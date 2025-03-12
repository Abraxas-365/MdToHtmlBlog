use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use log::{debug, error};
use tree_sitter::{Node, Parser};

use crate::apierror::ApiError;

use super::error::RendererError;

pub struct Renderer {
    parser: Parser,
    base_path: PathBuf,
    template_path: PathBuf,
}

impl Renderer {
    pub fn new<P: AsRef<Path>>(base_path: P, template_path: P) -> Result<Self, ApiError> {
        let mut parser = Parser::new();
        let markdown_language = tree_sitter_markdown::language();

        parser
            .set_language(markdown_language)
            .map_err(|e| ApiError::internal_error(e.to_string()))?;

        let base_path = base_path.as_ref().to_path_buf();
        let template_path = template_path.as_ref().to_path_buf();

        if !base_path.exists() {
            return Err(ApiError::not_found(format!(
                "Blog directory not found: {}",
                base_path.display()
            )));
        }

        if !template_path.exists() {
            return Err(ApiError::not_found(format!(
                "Template file not found: {}",
                template_path.display()
            )));
        }

        Ok(Renderer {
            parser,
            base_path,
            template_path,
        })
    }

    pub fn render(&self, path: &str) -> Result<String, ApiError> {
        // Get markdown content
        let md_path = self.resolve_markdown_path(path)?;
        let markdown_content =
            fs::read_to_string(&md_path).map_err(|e| RendererError::FileReadError {
                path: md_path.to_string_lossy().to_string(),
                source: e,
            })?;

        // Get template content
        let template =
            fs::read_to_string(&self.template_path).map_err(|e| RendererError::FileReadError {
                path: self.template_path.to_string_lossy().to_string(),
                source: e,
            })?;

        // Create a new parser for this request
        let mut parser = Parser::new();
        let markdown_language = tree_sitter_markdown::language();
        parser
            .set_language(markdown_language)
            .map_err(|e| RendererError::LanguageError(Box::new(e)))?;

        // Parse markdown
        let tree = parser.parse(&markdown_content, None).ok_or_else(|| {
            RendererError::MarkdownParseError(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to parse markdown",
            )))
        })?;

        let root_node = tree.root_node();

        // Extract metadata and references
        let metadata = extract_metadata(&markdown_content);
        let mut link_references = HashMap::new();
        extract_link_references(&root_node, &markdown_content, &mut link_references);

        // Convert to HTML and apply template
        let content_html = markdown_to_html(&root_node, &markdown_content, &link_references);
        let final_html = apply_template(&template, content_html, metadata);

        Ok(final_html)
    }

    fn resolve_markdown_path(&self, path: &str) -> Result<PathBuf, RendererError> {
        let clean_path = path
            .trim_start_matches('/')
            .trim_start_matches("blog/")
            .trim_end_matches('/');

        let mut full_path = self.base_path.clone();

        if clean_path.is_empty() {
            full_path.push("index.md");
        } else {
            full_path.push(format!("{}.md", clean_path));
        }

        debug!("Resolved markdown path: {:?}", full_path);

        if !full_path.exists() {
            error!("File not found: {:?}", full_path);
            return Err(RendererError::FileReadError {
                path: full_path.to_string_lossy().to_string(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
            });
        }

        if !full_path.starts_with(&self.base_path) {
            error!("Path traversal attempt detected");
            return Err(RendererError::InvalidPathError(
                "Path traversal not allowed".to_string(),
            ));
        }

        Ok(full_path)
    }
}

fn extract_metadata(markdown: &str) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_comment_block = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.starts_with("<!--") && !in_comment_block {
            in_comment_block = true;

            if trimmed.ends_with("-->") {
                parse_metadata_line(&trimmed[4..trimmed.len() - 3], &mut metadata);
                in_comment_block = false;
            }
            continue;
        }

        if trimmed.ends_with("-->") && in_comment_block {
            in_comment_block = false;
            continue;
        }

        if in_comment_block {
            parse_metadata_line(trimmed, &mut metadata);
            continue;
        }

        if !trimmed.is_empty() && !in_comment_block {
            break;
        }
    }

    metadata
}

fn parse_metadata_line(line: &str, metadata: &mut HashMap<String, String>) {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() == 2 {
        let key = parts[0].trim().to_lowercase();
        let value = parts[1].trim();
        metadata.insert(key, value.to_string());
    }
}

fn extract_link_references(
    node: &Node,
    source: &str,
    references: &mut HashMap<String, (String, String)>,
) {
    if node.kind() == "link_reference_definition" {
        let mut label = "";
        let mut destination = "";
        let mut title = "";

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "link_label" => {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            label = text.trim_start_matches('[').trim_end_matches(']');
                        }
                    }
                    "link_destination" => {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            destination = text;
                        }
                    }
                    "link_title" => {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            title = text.trim_matches('"').trim_matches('\'');
                        }
                    }
                    _ => {}
                }
            }
        }

        if !label.is_empty() && !destination.is_empty() {
            references.insert(
                label.to_lowercase(),
                (destination.to_string(), title.to_string()),
            );
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_link_references(&child, source, references);
        }
    }
}

fn apply_template(
    template: &str,
    content_html: String,
    metadata: HashMap<String, String>,
) -> String {
    let mut result = template.to_string();

    if let Some(title) = metadata.get("title") {
        result = result.replace("{title}", title);
    } else {
        result = result.replace("{title}", "Blog Post");
    }

    for (key, value) in metadata {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, &value);
    }

    result = result.replace("{content}", &content_html);

    result
}

fn convert_node_to_html(
    node: &Node,
    source: &str,
    html: &mut String,
    link_references: &HashMap<String, (String, String)>,
    current_list_key: &mut Option<String>,
    is_first_heading: &mut bool,
    is_first_paragraph: &mut bool,
) {
    match node.kind() {
        "document" => {
            let mut i = 0;
            while i < node.child_count() {
                if let Some(child) = node.child(i) {
                    match child.kind() {
                        "list" | "list_item" | "list_marker" => {
                            i += 1;
                            continue;
                        }
                        _ => {
                            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                                let trimmed = text.trim();

                                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                                    html.push_str("<ul class=\"space-y-2\">\n");
                                    process_list_items(text, html, false);
                                    html.push_str("</ul>\n");
                                } else if trimmed
                                    .chars()
                                    .next()
                                    .map_or(false, |c| c.is_ascii_digit())
                                    && trimmed.contains(". ")
                                {
                                    html.push_str("<ol class=\"space-y-2\">\n");
                                    process_list_items(text, html, true);
                                    html.push_str("</ol>\n");
                                } else {
                                    convert_node_to_html(
                                        &child,
                                        source,
                                        html,
                                        link_references,
                                        current_list_key,
                                        is_first_heading,
                                        is_first_paragraph,
                                    );
                                }
                            }
                        }
                    }
                }
                i += 1;
            }
        }

        "atx_heading" => {
            let mut level = 1;

            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind().contains("atx_h") && child.kind().contains("_marker") {
                        if let Ok(marker_text) = child.utf8_text(source.as_bytes()) {
                            level = marker_text.len();
                            break;
                        }
                    }
                }
            }

            let mut heading_content = String::new();
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "heading_content" {
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            heading_content = text.to_string();
                            break;
                        }
                    }
                }
            }

            if level == 1 && *is_first_heading {
                *is_first_heading = false;

                let github_url = "https://github.com/Abraxas-365";
                let linkedin_url =
                    "https://www.linkedin.com/in/luis-fernando-miranda-castillo-265b22203";
                let twitter_url = "#";

                html.push_str("<div class=\"flex flex-col md:flex-row justify-between items-start md:items-center\">\n");

                html.push_str(&format!(
                    "<h1 class=\"text-2xl text-gruvbox-yellow font-normal mt-8 mb-6 relative\">{}</h1>\n",
                    heading_content
                ));

                html.push_str(&format!(
                    r#"<div class="flex space-x-3 mb-6 md:mb-0 md:mt-8">
                        <a href="{}" target="_blank" class="text-gruvbox-blue hover:text-gruvbox-aqua">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none"
                                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                                class="lucide lucide-github">
                                <path
                                    d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4">
                                </path>
                                <path d="M9 18c-4.51 2-5-2-7-2"></path>
                            </svg>
                        </a>
                        <a href="{}" target="_blank" class="text-gruvbox-blue hover:text-gruvbox-aqua">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none"
                                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                                class="lucide lucide-linkedin">
                                <path d="M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-2-2 2 2 0 0 0-2 2v7h-4v-7a6 6 0 0 1 6-6z"></path>
                                <rect width="4" height="12" x="2" y="9"></rect>
                                <circle cx="4" cy="4" r="2"></circle>
                            </svg>
                        </a>
                        <a href="{}" class="text-gruvbox-blue hover:text-gruvbox-aqua">
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none"
                                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                                class="lucide lucide-twitter">
                                <path
                                    d="M22 4s-.7 2.1-2 3.4c1.6 10-9.4 17.3-18 11.6 2.2.1 4.4-.6 6-2C3 15.5.5 9.6 3 5c2.2 2.6 5.6 4.1 9 4-.9-4.2 4-6.6 7-3.8 1.1 0 3-1.2 3-1.2z">
                                </path>
                            </svg>
                        </a>
                    </div>"#,
                    github_url, linkedin_url, twitter_url
                ));

                html.push_str("</div>\n");
            } else {
                html.push_str(&format!(
                    "<h{} class=\"text-{} text-gruvbox-yellow font-normal mt-8 mb-6 relative\">",
                    level,
                    match level {
                        1 => "2xl",
                        2 => "xl",
                        3 => "lg",
                        _ => "base",
                    }
                ));

                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.kind() == "heading_content" {
                            for j in 0..child.child_count() {
                                if let Some(content_child) = child.child(j) {
                                    convert_node_to_html(
                                        &content_child,
                                        source,
                                        html,
                                        link_references,
                                        current_list_key,
                                        is_first_heading,
                                        is_first_paragraph,
                                    );
                                }
                            }
                        }
                    }
                }

                html.push_str(&format!("</h{}>\n", level));
            }
        }
        "paragraph" => {
            if current_list_key.is_some() {
                return;
            }

            if *is_first_paragraph && !*is_first_heading {
                *is_first_paragraph = false;

                let mut para_text = String::new();
                if let Ok(text) = node.utf8_text(source.as_bytes()) {
                    para_text = text.to_string();
                } else {
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                                para_text.push_str(text);
                            }
                        }
                    }
                }

                html.push_str(&format!("<p class=\"cursor\">{}</p>\n", para_text));
                html.push_str("<hr class=\"border-t border-gruvbox-gray my-8\">\n");
                return;
            }

            html.push_str("<p class=\"my-4\">");

            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    convert_node_to_html(
                        &child,
                        source,
                        html,
                        link_references,
                        current_list_key,
                        is_first_heading,
                        is_first_paragraph,
                    );
                }
            }

            html.push_str("</p>\n");
        }
        "link" => {
            let mut url = "";
            let mut text = "";
            let mut title = "";

            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    match child.kind() {
                        "link_text" => {
                            if let Ok(content) = child.utf8_text(source.as_bytes()) {
                                text = content;
                            }
                        }
                        "link_destination" => {
                            if let Ok(content) = child.utf8_text(source.as_bytes()) {
                                url = content;
                            }
                        }
                        "link_title" => {
                            if let Ok(content) = child.utf8_text(source.as_bytes()) {
                                title = content.trim_matches('"').trim_matches('\'');
                            }
                        }
                        _ => {}
                    }
                }
            }

            html.push_str(&format!(
                r#"<a href="{}" title="{}" class="text-gruvbox-blue hover:text-gruvbox-aqua">{}</a>"#,
                url,
                title,
                text
            ));
        }

        "image" => {
            let mut url = String::new();
            let mut alt = String::new();
            let mut title = String::new();
            let mut width = None;
            let mut height = None;
            let mut classes = String::from("max-w-full h-auto my-4 rounded-lg shadow-lg");
            let mut style = String::new();

            if let Ok(raw_text) = node.utf8_text(source.as_bytes()) {
                if let Some(url_start) = raw_text.find('(') {
                    if let Some(url_end) = raw_text[url_start..].find(')') {
                        url = raw_text[url_start + 1..url_start + url_end]
                            .trim()
                            .to_string();
                    }
                }

                if let Some(alt_start) = raw_text.find('[') {
                    if let Some(alt_end) = raw_text[alt_start..].find(']') {
                        alt = raw_text[alt_start + 1..alt_start + alt_end]
                            .trim()
                            .to_string();
                    }
                }

                if let Some(title_start) = raw_text.rfind('"') {
                    if let Some(title_end) = raw_text[..title_start].rfind('"') {
                        title = raw_text[title_end + 1..title_start].trim().to_string();
                    }
                }
            }

            let start_byte = node.start_byte();
            let preceding_text = &source[..start_byte];

            if let Some(last_comment_start) = preceding_text.rfind("<!--") {
                if let Some(comment_end) = preceding_text[last_comment_start..].find("-->") {
                    let comment =
                        &preceding_text[last_comment_start..last_comment_start + comment_end];
                    let attrs = comment
                        .trim_start_matches("<!--")
                        .trim_end_matches("-->")
                        .trim();

                    let mut current_attr = String::new();
                    let mut in_quotes = false;

                    let mut full_attrs = Vec::new();
                    for c in attrs.chars() {
                        match c {
                            '"' => {
                                current_attr.push(c);
                                in_quotes = !in_quotes;
                            }
                            ' ' if !in_quotes => {
                                if !current_attr.is_empty() {
                                    full_attrs.push(current_attr.clone());
                                    current_attr.clear();
                                }
                            }
                            _ => current_attr.push(c),
                        }
                    }
                    if !current_attr.is_empty() {
                        full_attrs.push(current_attr);
                    }

                    for attr in full_attrs {
                        if let Some((key, value)) = attr.split_once('=') {
                            let key = key.trim();
                            let value = value.trim();

                            match key {
                                "width" => width = Some(value.trim_matches('"').to_string()),
                                "height" => height = Some(value.trim_matches('"').to_string()),
                                "class" => {
                                    classes.push_str(&format!(" {}", value.trim_matches('"')))
                                }
                                "style" => {
                                    if value.starts_with('"') && value.ends_with('"') {
                                        style = value[1..value.len() - 1].to_string();
                                    } else {
                                        style = value.to_string();
                                    }
                                }
                                "preset" => {
                                    classes = match value.trim_matches('"') {
                                        "avatar" => {
                                            "w-32 h-32 rounded-full object-cover".to_string()
                                        }
                                        "banner" => "w-full h-64 object-cover".to_string(),
                                        "thumbnail" => "w-48 h-48 object-cover rounded".to_string(),
                                        _ => classes,
                                    }
                                }
                                _ => debug!("Unknown image attribute: {}={}", key, value),
                            }
                        }
                    }
                }
            }

            let mut img_tag = format!(
                r#"<img src="{}" alt="{}" title="{}" class="{}""#,
                url, alt, title, classes
            );

            if let Some(w) = width {
                img_tag.push_str(&format!(r#" width="{}""#, w));
            }
            if let Some(h) = height {
                img_tag.push_str(&format!(r#" height="{}""#, h));
            }
            if !style.is_empty() {
                img_tag.push_str(&format!(r#" style="{}""#, style));
            }

            img_tag.push('>');
            html.push_str(&img_tag);
        }

        "strong_emphasis" | "emphasis" => {
            let tag = if node.kind() == "strong_emphasis" {
                "strong"
            } else {
                "em"
            };
            let class = if node.kind() == "strong_emphasis" {
                "font-bold"
            } else {
                "italic"
            };

            html.push_str(&format!("<{} class=\"{}\">", tag, class));

            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() != "emphasis_delimiter" {
                        convert_node_to_html(
                            &child,
                            source,
                            html,
                            link_references,
                            current_list_key,
                            is_first_heading,
                            is_first_paragraph,
                        );
                    }
                }
            }

            html.push_str(&format!("</{}>", tag));
        }
        "code" | "code_span" => {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                html.push_str(&format!(
                    r#"<code class="bg-gruvbox-bg1 text-gruvbox-yellow px-2 py-1 rounded font-mono text-sm">{}</code>"#,
                    text
                ));
            }
        }

        "fenced_code_block" => {
            let mut language = "plaintext";
            let mut code_content = String::new();

            // Extract language and code content
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    match child.kind() {
                        "info_string" => {
                            if let Ok(lang) = child.utf8_text(source.as_bytes()) {
                                language = lang.trim();
                            }
                        }
                        "code_fence_content" => {
                            if let Ok(content) = child.utf8_text(source.as_bytes()) {
                                code_content = content.trim().to_string();
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Escape HTML special characters
            let escaped_content = code_content
                .replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("\"", "&quot;")
                .replace("'", "&#39;");

            html.push_str(&format!(
                r#"<pre class="line-numbers"><code class="language-{}">{}</code></pre>"#,
                language, escaped_content
            ));
        }

        "block_quote" => {
            html.push_str(
                r#"<blockquote class="border-l-4 border-gruvbox-gray pl-4 my-4 italic">"#,
            );

            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    convert_node_to_html(
                        &child,
                        source,
                        html,
                        link_references,
                        current_list_key,
                        is_first_heading,
                        is_first_paragraph,
                    );
                }
            }

            html.push_str("</blockquote>");
        }

        // Default case
        _ => {
            if !is_list_node(node) {
                if let Ok(text) = node.utf8_text(source.as_bytes()) {
                    html.push_str(text);
                } else {
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            convert_node_to_html(
                                &child,
                                source,
                                html,
                                link_references,
                                current_list_key,
                                is_first_heading,
                                is_first_paragraph,
                            );
                        }
                    }
                }
            }
        }
    }
}
fn markdown_to_html(
    node: &Node,
    source: &str,
    link_references: &HashMap<String, (String, String)>,
) -> String {
    let mut html = String::new();
    let mut is_first_heading = true;
    let mut is_first_paragraph = true;
    let mut current_list_key = None;

    convert_node_to_html(
        node,
        source,
        &mut html,
        link_references,
        &mut current_list_key,
        &mut is_first_heading,
        &mut is_first_paragraph,
    );

    html
}

fn is_list_node(node: &Node) -> bool {
    let kind = node.kind();
    kind == "list" || kind == "list_item" || kind.contains("list_marker")
}

fn process_list_items(text: &str, html: &mut String, is_ordered: bool) {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if is_ordered {
            if let Some(pos) = trimmed.find(". ") {
                let content = trimmed[pos + 2..].trim();
                html.push_str(&format!("<li>{}</li>\n", content));
            }
        } else {
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                let content = trimmed[2..].trim();
                html.push_str(&format!("<li>{}</li>\n", content));
            }
        }
    }
}

unsafe impl Send for Renderer {}
unsafe impl Sync for Renderer {}

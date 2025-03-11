use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use tree_sitter::{Node, Parser};

// Function to extract metadata from markdown comments at the top
fn extract_metadata(markdown: &str) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    // Look for HTML comments at the start of the file
    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_comment_block = false;

    for line in lines {
        let trimmed = line.trim();

        // Start of comment block
        if trimmed.starts_with("<!--") && !in_comment_block {
            in_comment_block = true;

            // Handle single-line comments
            if trimmed.ends_with("-->") {
                parse_metadata_line(&trimmed[4..trimmed.len() - 3], &mut metadata);
                in_comment_block = false;
            }
            continue;
        }

        // End of comment block
        if trimmed.ends_with("-->") && in_comment_block {
            in_comment_block = false;
            continue;
        }

        // Inside comment block
        if in_comment_block {
            parse_metadata_line(trimmed, &mut metadata);
            continue;
        }

        // Exit if we've passed the comment block at the top
        if !trimmed.is_empty() && !in_comment_block {
            break;
        }
    }

    metadata
}

// Parse a single line of metadata
fn parse_metadata_line(line: &str, metadata: &mut HashMap<String, String>) {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() == 2 {
        let key = parts[0].trim().to_lowercase();
        let value = parts[1].trim();
        metadata.insert(key, value.to_string());
    }
}

// Function to extract link references from the document
fn extract_link_references(
    node: &Node,
    source: &str,
    references: &mut HashMap<String, (String, String)>,
) {
    if node.kind() == "link_reference_definition" {
        let mut label = "";
        let mut destination = "";
        let mut title = "";

        // Extract link reference components
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

    // Recursively process children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_link_references(&child, source, references);
        }
    }
}

// Function to convert Markdown to HTML

// Updated main function
// Function
fn main() -> io::Result<()> {
    // Initialize the parser
    let mut parser = Parser::new();

    // Get the language for Markdown
    let markdown_language = tree_sitter_markdown::language();

    // Set the language
    parser
        .set_language(markdown_language)
        .expect("Error setting language");

    // File paths
    let markdown_path = "content.md";
    let template_path = "template.html";
    let output_path = "output.html";

    // Read the markdown file
    let markdown_content = fs::read_to_string(markdown_path)
        .expect(&format!("Failed to read markdown file: {}", markdown_path));

    // Read the template file
    let template = fs::read_to_string(template_path)
        .expect(&format!("Failed to read template file: {}", template_path));

    // Pre-process the AST for lists
    // This is a hack to capture list items directly from the markdown
    let lists = extract_list_items(&markdown_content);

    // Parse the markdown
    let tree = parser
        .parse(&markdown_content, None)
        .expect("Error parsing markdown");

    // Get the root node
    let root_node = tree.root_node();

    // Extract metadata from the markdown (like title from comments)
    let metadata = extract_metadata(&markdown_content);

    // Extract reference links for later resolution
    let mut link_references = HashMap::new();
    extract_link_references(&root_node, &markdown_content, &mut link_references);

    // Convert to HTML
    let content_html = markdown_to_html(&root_node, &markdown_content, &link_references, &lists);

    // Apply the template and generate the final HTML
    let final_html = apply_template(&template, content_html, metadata);

    // Write to file
    fs::write(output_path, final_html)?;
    println!("HTML generated successfully at: {}", output_path);

    Ok(())
}
fn apply_template(
    template: &str,
    content_html: String,
    metadata: HashMap<String, String>,
) -> String {
    let mut result = template.to_string();

    // Replace title placeholder with metadata or default
    if let Some(title) = metadata.get("title") {
        result = result.replace("{title}", title);
    } else {
        result = result.replace("{title}", "Blog Post");
    }

    // Replace other metadata placeholders
    for (key, value) in metadata {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, &value);
    }

    // Replace content placeholder
    result = result.replace("{content}", &content_html);

    result
}

fn convert_node_to_html(
    node: &Node,
    source: &str,
    html: &mut String,
    link_references: &HashMap<String, (String, String)>,
    lists: &HashMap<String, Vec<String>>,
    current_list_key: &mut Option<String>,
    is_first_heading: &mut bool,
    is_first_paragraph: &mut bool,
) {
    match node.kind() {
        "document" => {
            // Process all children of the document
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    // Skip link reference definitions as they're processed separately
                    if child.kind() != "link_reference_definition" {
                        convert_node_to_html(
                            &child,
                            source,
                            html,
                            link_references,
                            lists,
                            current_list_key,
                            is_first_heading,
                            is_first_paragraph,
                        );
                    }
                }
            }
        }
        "list" => {
            // Get the start line of the list
            let start_line = node.start_position().row;
            let list_key = start_line.to_string();

            // Check if we have pre-extracted list items for this list
            if lists.contains_key(&list_key) {
                *current_list_key = Some(list_key.clone());

                // Determine if ordered or unordered list
                let is_ordered = if let Some(first_item) = node.child(0) {
                    if first_item.kind() == "list_item" {
                        if let Some(marker) = first_item.child(0) {
                            if marker.kind() == "list_marker" {
                                if let Ok(marker_text) = marker.utf8_text(source.as_bytes()) {
                                    marker_text.chars().next().unwrap_or('-').is_ascii_digit()
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Generate the list HTML directly from our extracted items
                let tag = if is_ordered { "ol" } else { "ul" };
                html.push_str(&format!("<{} class=\"pl-6\">\n", tag));

                for item in &lists[&list_key] {
                    html.push_str(&format!("<li>{}</li>\n", item));
                }

                html.push_str(&format!("</{}>\n", tag));
            } else {
                // Fallback to regular processing
                let mut is_ordered = false;
                if let Some(first_item) = node.child(0) {
                    if first_item.kind() == "list_item" {
                        for i in 0..first_item.child_count() {
                            if let Some(child) = first_item.child(i) {
                                if child.kind() == "list_marker" {
                                    if let Ok(marker_text) = child.utf8_text(source.as_bytes()) {
                                        let first_char = marker_text.chars().next().unwrap_or(' ');
                                        is_ordered = first_char.is_ascii_digit();
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                let tag = if is_ordered { "ol" } else { "ul" };
                html.push_str(&format!("<{} class=\"pl-6\">\n", tag));
                html.push_str("<li>Fallback list processing</li>\n");
                html.push_str(&format!("</{}>\n", tag));
            }

            *current_list_key = None;
        }
        "atx_heading" => {
            // Get heading level by counting # characters in the marker
            let mut level = 1;

            // Look for the marker child
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

            // Extract heading content
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

            // Special handling for the first h1 heading
            if level == 1 && *is_first_heading {
                *is_first_heading = false;

                // Get social links from metadata or use defaults
                let github_url = "https://github.com/Abraxas-365";
                let linkedin_url =
                    "https://www.linkedin.com/in/luis-fernando-miranda-castillo-265b22203";
                let twitter_url = "#";

                // Start the custom header div
                html.push_str("<div class=\"flex flex-col md:flex-row justify-between items-start md:items-center\">\n");

                // Add name heading
                html.push_str(&format!(
                    "<h1 class=\"text-2xl text-gruvbox-yellow font-normal mt-8 mb-6 relative\">{}</h1>\n",
                    heading_content
                ));

                // Add social links
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

                // Close the custom header div
                html.push_str("</div>\n");
            } else {
                // Regular heading processing
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

                // Process heading content
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.kind() == "heading_content" {
                            // Process all children of the heading content
                            for j in 0..child.child_count() {
                                if let Some(content_child) = child.child(j) {
                                    convert_node_to_html(
                                        &content_child,
                                        source,
                                        html,
                                        link_references,
                                        lists,
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
            // Skip paragraph processing if inside a list, as we handle lists separately
            if current_list_key.is_some() {
                return;
            }

            // Special handling for the first paragraph (biography)
            if *is_first_paragraph && !*is_first_heading {
                *is_first_paragraph = false;

                // Extract paragraph text
                let mut para_text = String::new();
                if let Ok(text) = node.utf8_text(source.as_bytes()) {
                    para_text = text.to_string();
                } else {
                    // If we can't get the text directly, try each child
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                                para_text.push_str(text);
                            }
                        }
                    }
                }

                // Output as a bio paragraph with cursor effect
                html.push_str(&format!("<p class=\"cursor\">{}</p>\n", para_text));
                html.push_str("<hr class=\"border-t border-gruvbox-gray my-8\">\n");
                return;
            }

            // Regular paragraph
            html.push_str("<p class=\"my-4\">");

            // Process all children of the paragraph
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    convert_node_to_html(
                        &child,
                        source,
                        html,
                        link_references,
                        lists,
                        current_list_key,
                        is_first_heading,
                        is_first_paragraph,
                    );
                }
            }

            html.push_str("</p>\n");
        }
        // Handle other node types (text, emphasis, etc.)
        _ => {
            // Skip processing if we're inside a list item
            if current_list_key.is_some()
                && node.parent().map_or(false, |p| p.kind() == "list_item")
            {
                return;
            }

            // Try to get the text content directly
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                html.push_str(text);
            } else {
                // Process children for non-handled node types
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        convert_node_to_html(
                            &child,
                            source,
                            html,
                            link_references,
                            lists,
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
// Updated markdown_to_html function
fn markdown_to_html(
    node: &Node,
    source: &str,
    link_references: &HashMap<String, (String, String)>,
    lists: &HashMap<String, Vec<String>>,
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
        lists,
        &mut current_list_key,
        &mut is_first_heading,
        &mut is_first_paragraph,
    );

    html
}
// Simple HTML escape function
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn is_ancestor(node: &Node, ancestor_kind: &str) -> bool {
    let mut current = node.parent();

    while let Some(parent) = current {
        if parent.kind() == ancestor_kind {
            return true;
        }
        current = parent.parent();
    }

    false
}

// The most straightforward fix: Complete list item handling rewrite
fn fixed_list_item_handling(
    node: &Node,
    source: &str,
    html: &mut String,
    link_references: &HashMap<String, (String, String)>,
) {
    match node.kind() {
        "list" => {
            // Determine if ordered or unordered list
            let mut is_ordered = false;
            if let Some(first_item) = node.child(0) {
                if first_item.kind() == "list_item" {
                    if let Some(marker) = first_item.child(0) {
                        if marker.kind() == "list_marker" {
                            if let Ok(marker_text) = marker.utf8_text(source.as_bytes()) {
                                is_ordered =
                                    marker_text.chars().next().unwrap_or('-').is_ascii_digit();
                            }
                        }
                    }
                }
            }

            let tag = if is_ordered { "ol" } else { "ul" };
            html.push_str(&format!("<{} class=\"pl-6\">\n", tag));

            // Process each list item
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "list_item" {
                        html.push_str("<li>");

                        // Skip marker and process only the content
                        let mut marker_processed = false;
                        for j in 0..child.child_count() {
                            if let Some(item_child) = child.child(j) {
                                if item_child.kind() == "list_marker" {
                                    marker_processed = true;
                                    continue;
                                }

                                if marker_processed {
                                    // For paragraphs inside list items, we don't want the <p> tags
                                    if item_child.kind() == "paragraph" {
                                        for k in 0..item_child.child_count() {
                                            if let Some(para_child) = item_child.child(k) {
                                                if let Ok(text) =
                                                    para_child.utf8_text(source.as_bytes())
                                                {
                                                    html.push_str(text);
                                                }
                                            }
                                        }
                                    } else {
                                        // For other content types (like nested lists), process normally
                                        fixed_list_item_handling(
                                            &item_child,
                                            source,
                                            html,
                                            link_references,
                                        );
                                    }
                                }
                            }
                        }

                        html.push_str("</li>\n");
                    }
                }
            }

            html.push_str(&format!("</{}>\n", tag));
        }

        // For other node types, process them normally or just output their text
        "text" | "code" | "emphasis" | "strong_emphasis" => {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                html.push_str(text);
            }
        }

        _ => {
            // Process other node types by recursively processing their children
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    fixed_list_item_handling(&child, source, html, link_references);
                }
            }
        }
    }
}
fn is_in_list_item_context(node: &Node) -> bool {
    let mut current = Some(*node);

    // Check ancestors up to a certain depth to find a list_item
    let mut depth = 0;
    while let Some(n) = current {
        if n.kind() == "list_item" {
            return true;
        }

        // Get the parent node
        current = n.parent();

        // Limit depth to avoid infinite loop
        depth += 1;
        if depth > 10 {
            break;
        }
    }

    false
}
fn extract_list_items(markdown: &str) -> HashMap<String, Vec<String>> {
    let mut lists = HashMap::new();
    let mut current_list = Vec::new();
    let mut list_start_line = 0;
    let mut in_list = false;

    for (i, line) in markdown.lines().enumerate() {
        let trimmed = line.trim();

        // Check if line is a list item (starts with - or number.)
        if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || (trimmed.len() >= 3
                && trimmed.chars().next().unwrap().is_ascii_digit()
                && trimmed.chars().nth(1) == Some('.')
                && trimmed.chars().nth(2) == Some(' '))
        {
            if !in_list {
                // Start of a new list
                in_list = true;
                list_start_line = i;
                current_list.clear();
            }

            // Extract the list item text (removing the marker)
            let item_text = if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                trimmed[2..].trim().to_string()
            } else {
                // For numbered lists, find position after digit(s) and period
                let pos = trimmed.find(". ").unwrap_or(0) + 2;
                trimmed[pos..].trim().to_string()
            };

            current_list.push(item_text);
        } else if in_list && trimmed.is_empty() {
            // Empty line marks the end of a list
            if !current_list.is_empty() {
                lists.insert(list_start_line.to_string(), current_list.clone());
                current_list.clear();
            }
            in_list = false;
        } else if in_list && !trimmed.is_empty() {
            // If we encounter a non-empty, non-list-item line, it could be:
            // 1. A continuation of a list item (indented)
            // 2. The end of the list (not indented)

            if line.starts_with("  ") || line.starts_with("\t") {
                // Continuation of the previous list item
                if !current_list.is_empty() {
                    let last_idx = current_list.len() - 1;
                    current_list[last_idx] = format!("{} {}", current_list[last_idx], trimmed);
                }
            } else {
                // End of list
                if !current_list.is_empty() {
                    lists.insert(list_start_line.to_string(), current_list.clone());
                    current_list.clear();
                }
                in_list = false;
            }
        }
    }

    // Don't forget the last list if we reached the end of file while in a list
    if in_list && !current_list.is_empty() {
        lists.insert(list_start_line.to_string(), current_list);
    }

    lists
}

<!--
title: I Built My Own Markdown Blog Renderer (Because I Dont Know How To Exit Vim)
date: 2025-03-12
author: Luis Fernando Miranda
description: How I wasted a perfectly good weekend building a blog renderer instead of, you know, actually blogging
-->

# I Built My Own Markdown Blog Renderer (Because I Dont Know How To Exit Vim)

Let's be honest here. Normal people use Medium or WordPress to blog. But I'm a developer, so clearly I need to spend 20 hours building my own solution instead of writing actual content. Classic.

## Why I Did This To Myself
For years I've been saying "I should start a blog." The problem was:

1. *I write everything in Markdown* - Because I use Vim (did I mention I use Vim?), and anything else would mean leaving my precious terminal.

2. *Normal humans expect pretty blogs* - Apparently not everyone wants to read raw .md files. The audacity.

I looked at all the ready-made options. Jekyll? Too mainstream. WordPress? Please, I have standards. Hugo? Almost tempting, but it didn't give me the chance to overcomplicate things with Rust.

## The Birth of Yet Another Blog Engine

After my fifth coffee one night, I had what alcoholics call "a moment of clarity" and developers call "a terrible idea": I'll just build my own blog renderer! How hard could it be? (Narrator: It was harder than he thought.)

I picked Rust because:
- I wanted compile errors to remind me of my human failings
- Saying "I'm writing a Rust backend" sounds way cooler at meetups

## The Technical Stuff (Without the Fancy Words)

The core of this thing uses [tree-sitter](https://tree-sitter.github.io/tree-sitter/) to read Markdown. It's like a smart parser that doesn't choke when I mess up my syntax:

```rust
// Here's where the program either works or tells me I'm stupid
let tree = parser.parse(&markdown_content, None).ok_or_else(|| {
    RendererError::MarkdownParseError(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Your Markdown is bad and you should feel bad",
    )))
})?;
```


### Hiding Post Info in Comments

I stuck all the post info in HTML comments at the top of each file:

```markdown
<!--
title: Why Vim Is Better Than Your Editor
date: 2025-03-12
ego_boost: maximum
-->
```


The program finds these and puts them where they belong in the HTML. It's like having a secretary who actually reads your notes.

### Templates Because I'm Lazy

Each post gets wrapped in a template because typing HTML headers makes me want to cry:

```rust
let final_html = apply_template(&template, content_html, metadata);
// Now I can pretend HTML doesn't exist
```


## When Reality Hit Hard

### Tree Traversal: The Nightmare

Have you ever tried to walk through a tree structure in code? It's like trying to help your drunk friend find their way home after a party:

```rust
fn convert_node_to_html(node: &Node, /* more stuff */) {
    match node.kind() {
        "document" => { /* do stuff here */ },
        "heading" => { /* make bigger text I guess */ },
        _ => { /* cry a little */ }
    }
}
```



## It Actually Works (Shocking, I Know)

After many hours of swearing at my computer (with Vim open, of course), I ended up with a blog renderer that:

- Lets me write posts in Vim using Markdown like a proper snob
- Makes HTML that doesn't look like it's from 1997
- Works fast enough that nobody will complain
- Hasn't crashed yet (the bar is low)

## Future Stuff I'll Probably Never Get Around To

Because no project is ever really "done," here's what I'm pretending I'll add someday:

- Caching (to make my blog load 0.01 seconds faster)
- Search (for when I have more than two posts, which might be never)

## The Lesson Here

Building this blog engine taught me several important things:

1. There are approximately 10,000 existing solutions to this problem
2. None of them gave me the ego boost of building my own
3. I've now spent 5x more time on the engine than on actual blog content

But honestly, there's something satisfying about making your own tools. It's like that time I spent three days configuring my Vim setup to save approximately 2 seconds per day in productivity. Totally worth it.

If you want to see this mess of code, check out the [GitHub repo](https://github.com/Abraxas-365/MdToHtmlBlog). Feel free to star it, fork it, or use it as an example of what not to do.

Now that I've built a whole engine, I guess I need to write some actual blog posts. Coming up next: "My .vimrc Is Better Than Yours: A Detailed Analysis."

[Link to GitHub Repository](https://github.com/Abraxas-365/MdToHtmlBlog)


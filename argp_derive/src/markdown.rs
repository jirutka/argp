use pulldown_cmark::{Event, Parser, Tag};

struct MarkdownToPlainText {
    /// The output buffer.
    buf: String,

    /// A counter that keeps track of a block quote nesting level.
    block_quote_level: usize,

    /// A stack that keeps track of (nested) lists and the current numbering
    /// within ordered lists.
    list_stack: Vec<Option<u64>>,

    /// If set to `true`, only `<br>` can write a newline. It's initially set to
    /// `true` to suppress leading newlines and every time `<br>` is
    /// encountered. It's set to `false` when any character is written to the
    /// buffer (via `self.write()`).
    suppress_newlines: bool,

    /// If set to `true`, nothing is written to the buffer via `self.write()`
    /// and `self.write_newline()` methods. This is used to suppress link titles
    /// (`[<title>](<url>)`).
    suppress_output: bool,
}

impl MarkdownToPlainText {
    fn new() -> Self {
        Self {
            buf: Default::default(),
            block_quote_level: 0,
            list_stack: Default::default(),
            suppress_newlines: true,
            suppress_output: false,
        }
    }

    fn convert(mut self, markdown: &str) -> String {
        for event in Parser::new(markdown) {
            use Event::*;
            match event {
                // The start and end events don't contain the text inside the tag.
                // That's handled by the `Event::Text` arm.
                Start(tag) => {
                    self.start_tag(tag);
                }
                End(tag) => {
                    self.end_tag(tag);
                }
                Text(content) => {
                    self.write(&content);
                }
                Code(content) => {
                    self.write("`");
                    self.write(&content);
                    self.write("`");
                }
                Html(content) => match content.as_ref() {
                    "<br>" | "<br/>" => {
                        self.buf.push('\n');
                        self.suppress_newlines = true;
                    }
                    _ => self.write(&content),
                },
                SoftBreak if !self.suppress_newlines => {
                    self.write(" ");
                }
                HardBreak => {
                    self.write_newline();
                }
                Rule => {
                    self.write_newline();
                    self.write("---");
                    self.write_newline();
                }
                _ => (),
            }
        }
        self.buf.trim_end().to_string()
    }

    fn start_tag(&mut self, tag: Tag) {
        use Tag::*;
        match &tag {
            Paragraph | CodeBlock(_) | Heading(_, _, _) => {
                self.write_newline();
            }
            BlockQuote => {
                self.block_quote_level += 1;
            }
            Strong => {
                self.write("*");
            }
            Link(_, url, _) => {
                self.write(url);
                self.suppress_output = true; // don't write the link title
            }
            Item => {
                self.write_newline();
                self.write_list_indent();

                if let Some(Some(num)) = self.list_stack.last_mut() {
                    let serial = format!("{}. ", num);
                    *num += 1;
                    self.write(&serial);
                } else {
                    self.write("* ");
                }
            }
            List(num) => {
                self.list_stack.push(*num);
            }
            _ => (),
        }
    }

    fn end_tag(&mut self, tag: Tag) {
        use Tag::*;
        match &tag {
            Paragraph | Heading(_, _, _) => {
                self.write_newline();
            }
            BlockQuote => {
                self.block_quote_level -= 1;
            }
            Strong => {
                self.write("*");
            }
            CodeBlock(_) if !self.buf.ends_with('\n') => {
                self.write_newline();
            }
            Link(_, _, _) => {
                self.suppress_output = false;
            }
            List(_) => {
                self.list_stack.pop();
                if self.list_stack.is_empty() {
                    self.write_newline()
                }
            }
            _ => (),
        }
    }

    fn write_list_indent(&mut self) {
        if self.list_stack.is_empty() {
            return;
        }
        // Indent ordered lists with 3 spaces, unordered with 2 spaces.
        let width = self.list_stack[..self.list_stack.len() - 1]
            .iter()
            .map(|list| if list.is_some() { 3 } else { 2 })
            .sum();
        self.write(&" ".repeat(width));
    }

    #[inline]
    fn write_newline(&mut self) {
        if !self.suppress_newlines && !self.suppress_output {
            self.buf.push('\n');
        }
    }

    #[inline]
    fn write(&mut self, s: &str) {
        if self.suppress_output {
            return;
        }
        if self.block_quote_level > 0 && self.buf.ends_with('\n') {
            self.buf.push_str(&" ".repeat(self.block_quote_level * 2));
        }
        self.buf.push_str(s);
        self.suppress_newlines = false;
    }
}

/// Converts the given Markdown-formatted text into a plain text.
pub(crate) fn to_plain_text(markdown: &str) -> String {
    MarkdownToPlainText::new().convert(markdown)
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    macro_rules! test_case {
        ( $name:ident, $input:tt, $expected:tt ) => {
            #[test]
            fn $name() {
                assert_eq!(to_plain_text(indoc!($input)), indoc!($expected).trim_end());
            }
        };
    }

    test_case! { heading_1, "# Heading 1", "Heading 1" }

    test_case! { heading_2, "## Heading 2", "Heading 2" }

    test_case! { heading_3, "### Heading 3", "Heading 3" }

    test_case! { strong, "This is **strong** text", "This is *strong* text" }

    test_case! { emphasis, "This is *emphasised* text", "This is emphasised text" }

    test_case! { inline_code, "This is `inline` code", "This is `inline` code" }

    test_case! { link, "See [link](https://example.org)", "See https://example.org" }

    test_case! { unordered_lists, "
        - item 1
          + item 1.1
          + item 1.2
        - item 2
        - item 3
           * item 3.1
    ", "
        * item 1
          * item 1.1
          * item 1.2
        * item 2
        * item 3
          * item 3.1
    "}

    test_case! { ordered_lists, "
        1. item 1
            1. item 1.1
        2. item 2
        3. item 3
           1. item 3.1
           2. item 3.2
    ", "
        1. item 1
           1. item 1.1
        2. item 2
        3. item 3
           1. item 3.1
           2. item 3.2
    "}

    test_case! { mixed_lists, "
        1. item 1
           * item 1.1
              1. item 1.1.1
                 * item 1.1.1.1
           * item 1.2
              * item 1.2.1
        2. item 2
           1. item 2.1
    ", "
        1. item 1
           * item 1.1
             1. item 1.1.1
                * item 1.1.1.1
           * item 1.2
             * item 1.2.1
        2. item 2
           1. item 2.1
    "}

    test_case! { code_block, r#"
        ```
        # This is a code block

        print!("Hello");
        println!("world!");
        ```
    "#, r#"
        # This is a code block

        print!("Hello");
        println!("world!");
    "#}

    test_case! { block_quote, "
        A block quote follows:
        > This is inside a
        > block quote.
        >
        > Still in the quote.

        And this is below the quote.
    ", "
        A block quote follows:

          This is inside a block quote.

          Still in the quote.

        And this is below the quote.
    "}

    test_case! { block_quote_complex, "
        A block quote follows:
        > This is inside a<br>
        > block quote.
        >
        > * item
        >   * nested item
        >
        >> We need to go deeper.
        >>
        >>> Much deeper.

        And this is below the quote.
    ", "
        A block quote follows:

          This is inside a
          block quote.

          * item
            * nested item

            We need to go deeper.

              Much deeper.

        And this is below the quote.
    "}

    test_case! { html_tags, "<file> <b>bold</b>", "<file> <b>bold</b>" }

    test_case! { html_multiline, "
        <section>
          <h1>Hello</h1>
        </section>
    ", "
        <section>
          <h1>Hello</h1>
        </section>
    "}

    test_case! { horizontal_rule, "
        Before rule.

        - - -

        After rule.
    ", "
        Before rule.

        ---

        After rule.
    "}

    test_case! { soft_break, "
        This is a soft
        break
    ", "
        This is a soft break
    "}

    test_case! { hard_break, r#"
        This is a hard\
        break
    "#, "
        This is a hard
        break
    "}

    test_case! { br_before_list, "
        List of items:<br>
        * first
        * second
    ", "
        List of items:
        * first
        * second
    "}

    test_case! { br_br_inside_paragraph, "
        Lorem ipsum<br><br/>dolor sit amet.
    ", "
        Lorem ipsum

        dolor sit amet.
    "}

    test_case! { example, r#"
        # Title

        Lorem ipsum *dolor* sit **amet**, consectetur `adipiscing`
        elit. Sed do [`link`](https://example.org).

        See this code:
        ```
        # This is an example.

        print!("Hello");
        println!(" world!");
        ```

        There is a list:
        - item 1
        - item 2

        And a list without a blank line after this text:<br>
        1. item 1
        1. item 2

        > This is a block quote
        >
        > ...that contains a code:
        > ```
        > println!("Hey!");
        > ```
    "#, r#"
        Title

        Lorem ipsum dolor sit *amet*, consectetur `adipiscing` elit. Sed do https://example.org.

        See this code:

        # This is an example.

        print!("Hello");
        println!(" world!");

        There is a list:

        * item 1
        * item 2

        And a list without a blank line after this text:
        1. item 1
        2. item 2

          This is a block quote

          ...that contains a code:

          println!("Hey!");
    "#}
}

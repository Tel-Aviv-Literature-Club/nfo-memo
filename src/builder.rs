pub fn format_document(text: &str, action: &str, width: usize) -> String {
    let width = width.clamp(20, 160);
    match action {
        "left" => align(text, width, Alignment::Left),
        "center" => align(text, width, Alignment::Center),
        "right" => align(text, width, Alignment::Right),
        "single-box" => boxed(text, width, BoxStyle::Single),
        "double-box" => boxed(text, width, BoxStyle::Double),
        "heavy-box" => boxed(text, width, BoxStyle::Heavy),
        "title" => title_banner(text, width),
        "rule" => append_rule(text, width),
        _ => text.to_string(),
    }
}

enum Alignment {
    Left,
    Center,
    Right,
}
enum BoxStyle {
    Single,
    Double,
    Heavy,
}

#[allow(dead_code)]
pub fn build_visual_box(
    text: &str,
    style: &str,
    alignment: &str,
    box_width: usize,
    canvas_width: usize,
) -> String {
    let canvas_width = canvas_width.clamp(20, 160);
    let box_width = box_width.clamp(12, canvas_width);
    let style = match style {
        "double" => BoxStyle::Double,
        "heavy" => BoxStyle::Heavy,
        _ => BoxStyle::Single,
    };
    let alignment = match alignment {
        "center" => Alignment::Center,
        "right" => Alignment::Right,
        _ => Alignment::Left,
    };
    boxed_with_layout(text, canvas_width, box_width, style, alignment)
}

/// Render already-authored NFO literally, applying the selected box alphabet.
pub fn build_preview(text: &str, style: &str, canvas_width: usize) -> String {
    let canvas_width = canvas_width.clamp(20, 160);
    let already_framed = text
        .lines()
        .next()
        .is_some_and(|line| matches!(line.chars().next(), Some('╔' | '┌' | '┏')))
        && text
            .lines()
            .last()
            .is_some_and(|line| matches!(line.chars().next(), Some('╝' | '┘' | '┛')));
    if already_framed {
        return map_box_style(text, style);
    }
    let width = canvas_width;
    let (tl, tr, bl, br, horizontal, vertical) = match style {
        "single" => ('┌', '┐', '└', '┘', '─', '│'),
        "heavy" => ('┏', '┓', '┗', '┛', '━', '┃'),
        _ => ('╔', '╗', '╚', '╝', '═', '║'),
    };
    let mut out = vec![format!(
        "{tl}{}{tr}",
        horizontal.to_string().repeat(width - 2)
    )];
    let inner_width = width - 2;
    for line in text.split('\n') {
        for part in wrap_line(line, inner_width) {
            let part: String = part.chars().take(inner_width).collect();
            let remaining = inner_width.saturating_sub(part.chars().count());
            out.push(format!(
                "{vertical}{part}{}{vertical}",
                " ".repeat(remaining)
            ));
        }
    }
    out.push(format!(
        "{bl}{}{br}",
        horizontal.to_string().repeat(width - 2)
    ));
    map_box_style(&out.join("\n"), style)
}

fn map_box_style(text: &str, style: &str) -> String {
    text.chars()
        .map(|ch| match style {
            "single" => match ch {
                '╔' | '╗' | '╚' | '╝' | '╠' | '╣' => {
                    if ch == '╔' {
                        '┌'
                    } else if ch == '╗' {
                        '┐'
                    } else if ch == '╚' {
                        '└'
                    } else if ch == '╝' {
                        '┘'
                    } else if ch == '╠' {
                        '├'
                    } else {
                        '┤'
                    }
                }
                '║' => '│',
                '═' => '─',
                other => other,
            },
            "heavy" => match ch {
                '╔' | '┌' => '┏',
                '╗' | '┐' => '┓',
                '╚' | '└' => '┗',
                '╝' | '┘' => '┛',
                '╠' | '├' => '┣',
                '╣' | '┤' => '┫',
                '║' | '│' => '┃',
                '═' | '─' => '━',
                other => other,
            },
            _ => match ch {
                '┌' | '┏' => '╔',
                '┐' | '┓' => '╗',
                '└' | '┗' => '╚',
                '┘' | '┛' => '╝',
                '├' | '┣' => '╠',
                '┤' | '┫' => '╣',
                '│' | '┃' => '║',
                '─' | '━' => '═',
                other => other,
            },
        })
        .collect()
}

pub fn section_block(style: &str, width: usize, canvas_width: usize) -> String {
    let width = width.clamp(12, 160);
    let margin = " ".repeat(canvas_width.clamp(width, 160).saturating_sub(width) / 2);
    let (tl, tr, bl, br, left, right, horizontal, vertical) = match style {
        "single" => ('┌', '┐', '└', '┘', '├', '┤', '─', '│'),
        "heavy" => ('┏', '┓', '┗', '┛', '┣', '┫', '━', '┃'),
        _ => ('╔', '╗', '╚', '╝', '╠', '╣', '═', '║'),
    };
    let rule = horizontal.to_string().repeat(width.saturating_sub(2));
    let title = "SECTION TITLE";
    let pad = width.saturating_sub(title.chars().count() + 4);
    let title_left = pad / 2;
    let title_right = pad - title_left;
    let body = "Write section text here.";
    let body_right = width.saturating_sub(body.chars().count() + 3);
    format!(
        "\n\n{margin}{tl}{rule}{tr}\n{margin}{vertical}{}{title}{}{vertical}\n{margin}{left}{rule}{right}\n{margin}{vertical} {body}{}{vertical}\n{margin}{bl}{rule}{br}\n",
        " ".repeat(title_left + 1),
        " ".repeat(title_right + 1),
        " ".repeat(body_right)
    )
}

/// Align only the line containing the current text cursor.
pub fn align_current_line(text: &str, cursor: usize, width: usize, alignment: &str) -> String {
    let cursor = cursor.min(text.len());
    let cursor = (0..=cursor)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(0);
    let line_index = text[..cursor].bytes().filter(|&b| b == b'\n').count();
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    if let Some(line) = lines.get_mut(line_index) {
        let content = line.trim();
        let chars: Vec<char> = content.chars().collect();
        let framed = chars.len() >= 2
            && matches!(chars[0], '│' | '║' | '┃')
            && matches!(chars[chars.len() - 1], '│' | '║' | '┃');
        if framed {
            let inner: String = chars[1..chars.len() - 1].iter().collect();
            let inner = inner.trim();
            let inner_width = width.saturating_sub(4);
            let len = inner.chars().count().min(inner_width);
            let free = inner_width.saturating_sub(len);
            let left = match alignment {
                "center" => free / 2,
                "right" => free,
                _ => 0,
            };
            let right = free.saturating_sub(left);
            *line = format!(
                "{}{}{}{}{}",
                chars[0],
                " ".repeat(left + 1),
                inner,
                " ".repeat(right + 1),
                chars[chars.len() - 1]
            );
        } else {
            let len = content.chars().count();
            let pad = match alignment {
                "center" => width.saturating_sub(len) / 2,
                "right" => width.saturating_sub(len),
                _ => 0,
            };
            let left = pad;
            let right = width.saturating_sub(len + left);
            *line = format!("{}{content}{}", " ".repeat(left), " ".repeat(right));
        }
    }
    lines.join("\n")
}

/// Align a highlighted range inside its existing line, preserving the text around it.
pub fn align_selection(
    text: &str,
    anchor: usize,
    focus: usize,
    width: usize,
    alignment: &str,
) -> (String, usize) {
    let start = anchor.min(focus).min(text.len());
    let end = anchor.max(focus).min(text.len());
    let start = (0..=start)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(0);
    let end = (start..=end)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(start);
    if start == end || text[start..end].contains('\n') {
        return (align_current_line(text, focus, width, alignment), focus);
    }
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
    let prefix = &text[line_start..start];
    let selected = &text[start..end];
    let suffix = &text[end..line_end];
    let available = width.saturating_sub(prefix.chars().count() + suffix.chars().count());
    let free = available.saturating_sub(selected.chars().count());
    let left = match alignment {
        "center" => free / 2,
        "right" => free,
        _ => 0,
    };
    let right = free.saturating_sub(left);
    let replacement = format!(
        "{}{}{}{}{}",
        prefix,
        " ".repeat(left),
        selected,
        " ".repeat(right),
        suffix
    );
    let mut result = String::with_capacity(text.len() + free);
    result.push_str(&text[..line_start]);
    result.push_str(&replacement);
    result.push_str(&text[line_end..]);
    let caret = line_start + replacement.len();
    (result, caret)
}

pub fn box_current_line(
    text: &str,
    cursor: usize,
    width: usize,
    canvas_width: usize,
    style: &str,
    alignment: &str,
) -> String {
    let cursor = cursor.min(text.len());
    let cursor = (0..=cursor)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(0);
    let index = text[..cursor].bytes().filter(|&b| b == b'\n').count();
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let Some(line) = lines.get(index) else {
        return text.to_owned();
    };
    let width = width.clamp(12, 160);
    let margin = " ".repeat(canvas_width.clamp(width, 160).saturating_sub(width) / 2);
    let (tl, tr, bl, br, horizontal, vertical) = match style {
        "heavy" => ('┏', '┓', '┗', '┛', '━', '┃'),
        "single" => ('┌', '┐', '└', '┘', '─', '│'),
        _ => ('╔', '╗', '╚', '╝', '═', '║'),
    };
    let inner_width = width.saturating_sub(4);
    let mut block = vec![format!(
        "{margin}{tl}{}{tr}",
        horizontal.to_string().repeat(width - 2)
    )];
    for part in wrap_line(line.trim(), inner_width) {
        let free = inner_width.saturating_sub(part.chars().count());
        let left = match alignment {
            "center" => free / 2,
            "right" => free,
            _ => 0,
        };
        let right = free.saturating_sub(left);
        block.push(format!(
            "{margin}{vertical}{}{part}{}{vertical}",
            " ".repeat(left + 1),
            " ".repeat(right + 1)
        ));
    }
    block.push(format!(
        "{margin}{bl}{}{br}",
        horizontal.to_string().repeat(width - 2)
    ));
    lines.splice(index..=index, block);
    lines.join("\n")
}

fn align(text: &str, width: usize, alignment: Alignment) -> String {
    logical_lines(text)
        .into_iter()
        .flat_map(|line| wrap_line(line.trim(), width))
        .map(|line| {
            let len = line.chars().count();
            let margin = match alignment {
                Alignment::Left => 0,
                Alignment::Center => width.saturating_sub(len) / 2,
                Alignment::Right => width.saturating_sub(len),
            };
            format!("{}{}", " ".repeat(margin), line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn boxed(text: &str, canvas_width: usize, style: BoxStyle) -> String {
    let content_width = logical_lines(text)
        .iter()
        .map(|line| line.trim().chars().count())
        .max()
        .unwrap_or(0)
        .clamp(20, canvas_width.saturating_sub(4));
    let box_width = (content_width + 4).min(canvas_width);
    boxed_with_layout(text, canvas_width, box_width, style, Alignment::Left)
}

fn boxed_with_layout(
    text: &str,
    canvas_width: usize,
    box_width: usize,
    style: BoxStyle,
    alignment: Alignment,
) -> String {
    let inner_width = box_width.saturating_sub(4);
    let margin = " ".repeat(canvas_width.saturating_sub(box_width) / 2);
    let (tl, tr, bl, br, horizontal, vertical) = match style {
        BoxStyle::Single => ('┌', '┐', '└', '┘', '─', '│'),
        BoxStyle::Double => ('╔', '╗', '╚', '╝', '═', '║'),
        BoxStyle::Heavy => ('┏', '┓', '┗', '┛', '━', '┃'),
    };
    let mut output = vec![format!(
        "{margin}{tl}{}{tr}",
        horizontal.to_string().repeat(box_width - 2)
    )];
    for line in logical_lines(text)
        .into_iter()
        .flat_map(|line| wrap_line(line, inner_width))
    {
        let line = line.trim_end();
        let free = inner_width.saturating_sub(line.chars().count());
        let (left, right) = match alignment {
            Alignment::Left => (0, free),
            Alignment::Center => (free / 2, free - free / 2),
            Alignment::Right => (free, 0),
        };
        output.push(format!(
            "{margin}{vertical}{}{line}{}{vertical}",
            " ".repeat(left + 1),
            " ".repeat(right + 1)
        ));
    }
    output.push(format!(
        "{margin}{bl}{}{br}",
        horizontal.to_string().repeat(box_width - 2)
    ));
    output.join("\n")
}

fn title_banner(text: &str, width: usize) -> String {
    let title = logical_lines(text)
        .into_iter()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("TITLE")
        .trim();
    let title = truncate(title, width.saturating_sub(6));
    let inner = width.saturating_sub(2);
    let left = inner.saturating_sub(title.chars().count() + 2) / 2;
    let right = inner.saturating_sub(title.chars().count() + 2 + left);
    format!("╔{} {} {}╗", "═".repeat(left), title, "═".repeat(right))
}

fn append_rule(text: &str, width: usize) -> String {
    let prefix = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("{text}{prefix}{}", "─".repeat(width))
}

fn logical_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        vec![""]
    } else {
        text.lines().collect()
    }
}

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let chars: Vec<char> = line.chars().collect();
    chars
        .chunks(width.max(1))
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn truncate(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_a_box_inside_the_canvas() {
        let result = format_document("hello", "single-box", 40);
        let lines: Vec<_> = result.lines().collect();
        assert!(lines[0].starts_with("        "));
        assert_eq!(lines[0].chars().count(), 32);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn centers_text_at_requested_width() {
        assert_eq!(
            format_document("hello", "center", 40),
            format!("{}hello", " ".repeat(17))
        );
    }

    #[test]
    fn visual_box_centers_text_inside_a_centered_box() {
        let result = build_visual_box("hello", "double", "center", 24, 40);
        let lines: Vec<_> = result.lines().collect();
        assert_eq!(lines[0].chars().count(), 32);
        assert!(lines[0].starts_with("        ╔"));
        assert_eq!(lines[1].chars().count(), 32);
        assert!(lines[1].contains("║        hello         ║"));
    }
}

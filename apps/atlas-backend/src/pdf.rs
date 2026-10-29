/*!
 * FR-28 PDF export: a report as a plain monospaced table on A4 landscape, written by hand like `csv.rs` - a few
 * hundred bytes of PDF syntax per page doesn't justify a dependency. Text outside printable ASCII prints as `?`, and
 * a cell longer than a column is cut short with `~`; the CSV export carries the full values.
 */

const PAGE_WIDTH: f32 = 842.0;
const PAGE_HEIGHT: f32 = 595.0;
const MARGIN: f32 = 36.0;
const FONT_SIZE: f32 = 7.0;
const LEADING: f32 = 9.0;
/// Courier's advance is 0.6 em.
const CHAR_WIDTH: f32 = FONT_SIZE * 0.6;
const MAX_COLUMN: usize = 36;
const GAP: &str = "  ";

fn printable(text: &str) -> String {
    return text.chars().map(|c| if (' '..='~').contains(&c) { c } else { '?' }).collect();
}

fn fit(text: &str, width: usize) -> String {
    let text = printable(text);
    if text.len() <= width {
        return format!("{:<width$}", text, width = width);
    }
    return format!("{}~", &text[..width - 1]);
}

fn escape(text: &str) -> String {
    return text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
}

/// The lines of the table: header, rule, then one line per row, each cut to the page width.
fn layout(header: &[&str], rows: &[Vec<String>]) -> (Vec<String>, Vec<String>) {
    let mut widths: Vec<usize> = header.iter().map(|h| h.len().clamp(1, MAX_COLUMN)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(widths.len()) {
            widths[i] = widths[i].max(cell.chars().count().min(MAX_COLUMN));
        }
    }
    let max_chars = ((PAGE_WIDTH - 2.0 * MARGIN) / CHAR_WIDTH) as usize;
    let line = |cells: Vec<String>| -> String {
        let joined: String = cells.iter().zip(&widths).map(|(c, w)| fit(c, *w)).collect::<Vec<_>>().join(GAP);
        return joined.chars().take(max_chars).collect::<String>().trim_end().to_string();
    };
    let head = line(header.iter().map(|h| h.to_string()).collect());
    let rule = "-".repeat(head.len().min(max_chars));
    let body = rows.iter().map(|row| line(row.clone())).collect();
    return (vec![head, rule], body);
}

fn page_stream(title: &str, page: usize, pages: usize, head: &[String], lines: &[String]) -> String {
    let mut out = format!("BT /F1 {} Tf {} TL {} {} Td\n", FONT_SIZE, LEADING, MARGIN, PAGE_HEIGHT - MARGIN);
    out.push_str(&format!("({}) Tj T*\nT*\n", escape(&printable(&format!("{}  -  page {} of {}", title, page, pages)))));
    for text in head.iter().chain(lines) {
        out.push_str(&format!("({}) Tj T*\n", escape(text)));
    }
    out.push_str("ET\n");
    return out;
}

pub fn table(title: &str, header: &[&str], rows: &[Vec<String>]) -> Vec<u8> {
    let (head, body) = layout(header, rows);
    let per_page = (((PAGE_HEIGHT - 2.0 * MARGIN) / LEADING) as usize).saturating_sub(2 + head.len()).max(1);
    let chunks: Vec<&[String]> = if body.is_empty() { vec![&[][..]] } else { body.chunks(per_page).collect() };

    // Objects: 1 catalog, 2 page tree, 3 font, then a page and its content stream for each page
    let mut objects: Vec<String> = Vec::new();
    let kids: Vec<String> = (0..chunks.len()).map(|i| format!("{} 0 R", 4 + i * 2)).collect();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), chunks.len()));
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>".to_string());
    for (i, lines) in chunks.iter().enumerate() {
        let stream = page_stream(title, i + 1, chunks.len(), &head, lines);
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            PAGE_WIDTH, PAGE_HEIGHT, 5 + i * 2
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{}endstream", stream.len(), stream));
    }

    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", i + 1, object));
    }
    let xref = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for offset in offsets {
        pdf.push_str(&format!("{:010} 00000 n \n", offset));
    }
    pdf.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", objects.len() + 1, xref));
    return pdf.into_bytes();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_well_formed_pdf_with_a_correct_xref() {
        let rows = vec![vec!["Soap (bar)".to_string(), "12".to_string()], vec!["Rice".to_string(), "3".to_string()]];
        let pdf = String::from_utf8(table("Sales by product", &["product", "qty"], &rows)).unwrap();
        assert!(pdf.starts_with("%PDF-1.4\n") && pdf.ends_with("%%EOF\n"));
        assert!(pdf.contains("(Soap \\(bar\\)"));
        let xref: usize = pdf.rsplit("startxref\n").next().unwrap().lines().next().unwrap().parse().unwrap();
        assert!(pdf[xref..].starts_with("xref\n"));
        let first: usize = pdf[xref..].lines().nth(3).unwrap()[..10].parse().unwrap();
        assert!(pdf[first..].starts_with("1 0 obj"));
    }

    #[test]
    fn long_reports_split_across_pages_and_long_cells_are_cut() {
        let rows: Vec<Vec<String>> = (0..200).map(|i| vec![format!("{}", i), "x".repeat(80)]).collect();
        let pdf = String::from_utf8(table("Movements", &["n", "reason"], &rows)).unwrap();
        assert!(pdf.contains("/Count 4"));
        assert!(pdf.contains(&format!("{}~", "x".repeat(MAX_COLUMN - 1))));
    }
}

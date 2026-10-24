/*!
 * A small RFC 4180 reader for the import templates: commas, double-quoted fields with `""` escapes,
 * embedded newlines inside quotes, and CRLF or LF line endings.
 */

#[derive(Debug, PartialEq, Eq)]
pub struct CsvError(pub String);

/// Every record as its fields. Blank lines are skipped.
pub fn parse(text: &str) -> Result<Vec<Vec<String>>, CsvError> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => in_quotes = false,
                other => field.push(other),
            }
            continue;
        }
        match ch {
            '"' if field.is_empty() => in_quotes = true,
            ',' => record.push(std::mem::take(&mut field)),
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                record.push(std::mem::take(&mut field));
                push_record(&mut records, std::mem::take(&mut record));
            }
            other => field.push(other),
        }
    }
    if in_quotes {
        return Err(CsvError("a quoted field is never closed".to_string()));
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        push_record(&mut records, record);
    }
    return Ok(records);
}

fn push_record(records: &mut Vec<Vec<String>>, record: Vec<String>) {
    if record.iter().all(|f| f.trim().is_empty()) {
        return;
    }
    records.push(record);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_quoted_fields_escapes_and_line_endings() {
        let rows = parse("name,note\r\n\"Rice, 50kg\",\"say \"\"hi\"\"\"\nBeans,\"two\nlines\"\n\n").unwrap();
        assert_eq!(
            rows,
            vec![
                vec!["name".to_string(), "note".to_string()],
                vec!["Rice, 50kg".to_string(), "say \"hi\"".to_string()],
                vec!["Beans".to_string(), "two\nlines".to_string()],
            ]
        );
    }

    #[test]
    fn keeps_empty_trailing_fields_and_rejects_an_open_quote() {
        assert_eq!(parse("a,b,\n").unwrap(), vec![vec!["a".to_string(), "b".to_string(), String::new()]]);
        assert!(parse("a,\"b\n").is_err());
    }
}

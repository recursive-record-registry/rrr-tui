use std::borrow::Cow;

use itertools::Itertools;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Row,
};
use rrr::{
    cbor::{self, ValueExt},
    record::RecordMetadataKey,
};

fn styled<'a>(string: impl Into<Cow<'a, str>>) -> Span<'a> {
    Span::styled(string, Style::new().fg(Color::Rgb(0x3F, 0x3F, 0x3F)))
}

fn line<'a>(ty: impl AsRef<str>, content: impl Into<Line<'a>>) -> Line<'a> {
    let mut l: Line = content.into();
    l.spans.insert(0, styled(format!("{} ", ty.as_ref())));
    l
}

pub fn record_metadata_to_row<'a>(key: RecordMetadataKey<'a>, value: &'a cbor::Value) -> Row<'a> {
    match key {
        RecordMetadataKey::Id(id) => {
            Row::new([Line::raw(id.to_string()), cbor_value_to_line(value)])
        }
        RecordMetadataKey::Custom(key) => {
            Row::new([cbor_value_to_line(key.0), cbor_value_to_line(value)])
        }
    }
}

pub fn cbor_value_to_line(value: &cbor::Value) -> Line {
    if let Some(integer) = value.as_integer() {
        return line("integer", i128::from(integer).to_string());
    }

    if let Some(bytes) = value.as_bytes() {
        return line("bytes", format!("{:02x}", bytes.iter().format("")));
    }

    if let Some(float) = value.as_float() {
        return line("float", float.to_string());
    }

    if let Some(text) = value.as_text() {
        return line("text", text);
    }

    if let Some(boolean) = value.as_bool() {
        return line("bool", format!("{boolean:?}"));
    }

    if value.is_null() {
        return Line::from_iter([styled("(null)")]);
    }

    if let Some(array) = value.as_array() {
        return line(
            "array",
            Line::from_iter(
                array
                    .iter()
                    .map(|value| {
                        let mut spans = cbor_value_to_line(value).spans;
                        spans.insert(0, styled("{"));
                        spans.push(styled("}"));
                        spans
                    })
                    .intersperse_with(|| vec![Span::raw(" ")])
                    .flatten(),
            ),
        );
    }

    if let Some(map) = value.as_map() {
        return line(
            "map",
            Line::from_iter(
                map.iter()
                    .map(|(key, value)| {
                        let mut spans = cbor_value_to_line(key).spans;
                        spans.insert(0, styled("{"));
                        spans.push(styled("}: {"));
                        spans.append(&mut cbor_value_to_line(value).spans);
                        spans.push(styled("}"));
                        spans
                    })
                    .intersperse_with(|| vec![Span::raw(" ")])
                    .flatten(),
            ),
        );
    }

    if let Some(datetime) = value.as_datetime() {
        return line("datetime", datetime.to_rfc3339());
    }

    if let Some((tag, inner)) = value.as_tag() {
        return line(format!("tag({tag}) "), cbor_value_to_line(inner));
    }

    panic!("Unrecognized type of CBOR value: {value:?}");
}

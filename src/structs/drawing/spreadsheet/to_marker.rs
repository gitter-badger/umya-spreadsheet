// xdr:to
use writer::driver::*;
use quick_xml::events::{Event, BytesStart};
use quick_xml::Writer;
use quick_xml::Reader;
use std::io::Cursor;

#[derive(Clone, Default, Debug)]
pub struct ToMarker {
    col: usize,
    col_off: usize,
    row: usize,
    row_off: usize,
}
impl ToMarker {
    pub fn get_col(&self)-> &usize {
        &self.col
    }

    pub fn set_col(&mut self, value:usize)-> &mut ToMarker {
        self.col = value;
        self
    }

    pub fn get_col_off(&self)-> &usize {
        &self.col_off
    }

    pub fn set_col_off(&mut self, value:usize)-> &mut ToMarker {
        self.col_off = value;
        self
    }

    pub fn get_row(&self)-> &usize {
        &self.row
    }

    pub fn set_row(&mut self, value:usize)-> &mut ToMarker {
        self.row = value;
        self
    }

    pub fn get_row_off(&self)-> &usize {
        &self.row_off
    }

    pub fn set_row_off(&mut self, value:usize)-> &mut ToMarker {
        self.row_off = value;
        self
    }

    pub(crate) fn adjustment_insert_row(&mut self, num_rows:&usize) {
        self.row += num_rows;
    }

    pub(crate) fn adjustment_insert_column(&mut self, num_cols:&usize) {
        self.col += num_cols;
    }

    pub(crate) fn adjustment_remove_row(&mut self, num_rows:&usize) {
        self.row = if &self.row > num_rows { self.row - num_rows } else { 1 };
    }

    pub(crate) fn adjustment_remove_column(&mut self, num_cols:&usize) {
        self.col = if &self.col > num_cols { self.col - num_cols } else { 1 };
    }

    pub(crate) fn set_attributes<R: std::io::BufRead>(
        &mut self,
        reader:&mut Reader<R>,
        _e:&BytesStart,
    ) {
        let mut string_value:String = String::from("");
        let mut buf = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Text(e)) => string_value = e.unescape_and_decode(&reader).unwrap(),
                Ok(Event::End(ref e)) => {
                    match e.name() {
                        b"xdr:col" => {
                            self.col = string_value.parse::<usize>().unwrap();
                        },
                        b"xdr:colOff" => {
                            self.col_off = string_value.parse::<usize>().unwrap();
                        },
                        b"xdr:row" => {
                            self.row = string_value.parse::<usize>().unwrap();
                        },
                        b"xdr:rowOff" => {
                            self.row_off = string_value.parse::<usize>().unwrap();
                        },
                        b"xdr:to" => return,
                        _ => (),
                    }
                },
                Ok(Event::Eof) => panic!("Error not find {} end element", "xdr:to"),
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                _ => (),
            }
            buf.clear();
        }
    }

    pub(crate) fn write_to(&self, writer: &mut Writer<Cursor<Vec<u8>>>) {
        // xdr:to
        write_start_tag(writer, "xdr:to", vec![], false);

        // xdr:col
        write_start_tag(writer, "xdr:col", vec![], false);
        write_text_node(writer, &self.col.to_string());
        write_end_tag(writer, "xdr:col");

        // xdr:colOff
        write_start_tag(writer, "xdr:colOff", vec![], false);
        write_text_node(writer, &self.col_off.to_string());
        write_end_tag(writer, "xdr:colOff");
        
        // xdr:row
        write_start_tag(writer, "xdr:row", vec![], false);
        write_text_node(writer, &self.row.to_string());
        write_end_tag(writer, "xdr:row");

        // xdr:rowOff
        write_start_tag(writer, "xdr:rowOff", vec![], false);
        write_text_node(writer, &self.row_off.to_string());
        write_end_tag(writer, "xdr:rowOff");

        write_end_tag(writer, "xdr:to");
    }
}

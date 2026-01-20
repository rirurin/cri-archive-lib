use std::collections::HashMap;
use std::error::Error;
use std::io::{Cursor, Read, Seek, SeekFrom};
use crate::acb::header::HighTable;
use crate::schema::rows::RowValue;
use crate::schema::strings::{StringPool, StringPoolFast};

type Table = HighTable<StringPoolFast>;

#[derive(Debug)]
pub struct Waveform {
    awb_index: usize,
    size: usize
}

#[derive(Debug)]
pub struct Cue<'a> {
    name: &'a str,
    id: u32,
    waveforms: Vec<Waveform>,
}

#[derive(Debug)]
pub struct AcbReader {
    // raw stream
    stream: Vec<u8>,
    // archive tables
    header: Table,
    cue_tbl: Option<Table>,
    cue_name_tbl: Option<Table>,
    waveform_tbl: Option<Table>,
    sequence_tbl: Option<Table>,

    cue_name_to_index: HashMap<&'static str, usize>,
    cue_id_to_index: HashMap<u32, usize>,
}

impl AcbReader {
    fn build_cue_names(cue_name: &Table) -> Result<Vec<&'static str>, Box<dyn Error>> {
        let mut cue_names = Vec::with_capacity(cue_name.get_rows().len());
        for row in cue_name.get_rows() {
            if let Some(name) = cue_name.get_value(row, "CueName").and_then(|n| match n {
                RowValue::String(str) => cue_name.get_strings().get_string(*str),
                _ => None
            }) {
                cue_names.push(unsafe { std::str::from_utf8_unchecked(
                    std::slice::from_raw_parts(name.as_ptr(), name.len())) });
            }
        }
        Ok(cue_names)
    }

    fn get_table(header: &Table, name: &str) -> Result<Option<Table>, Box<dyn Error>> {
        let value = header.get_value_header(name);
        if value.is_none() {
            return Ok(None);
        }
        match value.unwrap() {
            RowValue::Data(data) => {
                let file_offset = (header.get_header().data_pool_offset() + data.get_offset()) as usize;
                Ok(Some(HighTable::new(
                    &header.get_slice()[file_offset..])?))
            },
            _ => Ok(None)
        }
    }

    pub fn new(stream: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        // let cursor = Cursor::new(stream.as_slice());
        let header = HighTable::new(stream.as_slice())?;
        let cue_tbl = Self::get_table(&header, "CueTable")?;
        let cue_name_tbl = Self::get_table(&header, "CueNameTable")?;
        let waveform_tbl = Self::get_table(&header, "WaveformTable")?;
        let sequence_tbl = Self::get_table(&header, "SequenceTable")?;

        let cue_name_to_index = match &cue_name_tbl {
            Some(tbl) => {
                let mut out = HashMap::new();
                for row in tbl.get_rows() {
                    if let Some(RowValue::String(name)) = tbl.get_value(row, "CueName") {
                        if let Some(name) = tbl.get_strings().get_string(*name) {
                            if let Some(RowValue::UInt16(index)) = tbl.get_value(row, "CueIndex") {
                                out.insert(unsafe { std::str::from_utf8_unchecked(
                                    std::slice::from_raw_parts(name.as_ptr(), name.len())) }, *index as usize);
                            }
                        }
                    }
                }
                out
            },
            None => HashMap::new()
        };

        let cue_id_to_index = match &cue_tbl {
            Some(tbl) => {
                let mut out = HashMap::new();
                for (i, row) in tbl.get_rows().iter().enumerate() {
                    if let Some(RowValue::UInt32(id)) = tbl.get_value(row, "CueId") {
                        out.insert(*id, i);
                    }
                }
                out
            },
            None => HashMap::new()
        };

        Ok(Self {
            stream,

            header,
            cue_tbl,
            cue_name_tbl,
            waveform_tbl,
            sequence_tbl,

            cue_name_to_index,
            cue_id_to_index
        })
    }

    pub fn get_name(&self) -> Option<&str> {
        let head = &self.header;
        head.get_value_header("Name").and_then(|v| match v {
            RowValue::String(s) => head.get_strings().get_string(*s),
            _ => None
        })
    }

    pub fn get_cue_by_name<'a>(&self, cue: &'a str) -> Option<Cue<'a>> {
        if self.cue_tbl.is_none() {
            return None;
        }
        self.cue_name_to_index.get(cue).and_then(|index| {
            let cue_tbl = self.cue_tbl.as_ref().unwrap();
            let cue_row = &cue_tbl.get_rows()[*index];
            match cue_tbl.get_value(cue_row, "CueId") {
                Some(RowValue::UInt32(cue_id)) => {
                    Some(Cue {
                        name: cue,
                        id: *cue_id,
                        waveforms: vec![]
                    })
                },
                _ => None
            }
        })
    }

    pub fn get_cue_by_id(&self, id: u32) -> Option<Cue<'_>> {
        self.cue_id_to_index.get(&id).and_then(|index| {
            let cue_name_tbl = self.cue_name_tbl.as_ref().unwrap();
            let cue_row = &cue_name_tbl.get_rows()[*index];
            cue_name_tbl.get_value(cue_row, "CueName").and_then(|v| match v {
                RowValue::String(str) => Some((cue_name_tbl, *str)),
                _ => None
            })
        }).and_then(|(tbl, row)| {
            tbl.get_strings().get_string(row).map(|name| {
                Cue {
                    name,
                    id,
                    waveforms: vec![]
                }
            })
        })
    }

    pub fn get_all_cue_names(&self) -> Vec<&str> {
        self.cue_name_to_index.keys().map(|v| *v).collect()
    }

    pub fn get_all_cue_ids(&self) -> Vec<u32> {
        self.cue_id_to_index.keys().map(|v| *v).collect()
    }
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use crate::acb::reader::AcbReader;

    #[test]
    fn read_metaphor_battle_voice() -> Result<(), Box<dyn Error>> {
        let sample_path = "E:/Metaphor/base_cpk/EN/sound/battle/character/bp01.acb";
        if !std::fs::exists(sample_path)? {
            return Ok(());
        }
        let reader = AcbReader::new(std::fs::read(sample_path)?)?;
        assert_eq!(reader.get_name(), Some("bp01"));
        let cue = reader.get_cue_by_name("v_bp_bp01_034_1_c001");
        assert_eq!(cue.is_some(), true);
        let cue = cue.unwrap();
        assert_eq!(cue.name, "v_bp_bp01_034_1_c001");
        assert_eq!(cue.id, 34);
        Ok(())
    }
}
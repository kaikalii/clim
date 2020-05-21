#![allow(dead_code)]

use std::io::Read;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("XML error: {0}")]
    XML(#[from] xmltree::ParseError),
    #[error("Missing element {0:?}")]
    Missing(String),
}

pub struct Fomod {}

impl Fomod {
    pub fn parse<I, C>(info: I, config: C) -> Result<Self, Error>
    where
        I: Read,
        C: Read,
    {
        let info_tree = xmltree::Element::parse(info)?;
        println!("{:#?}", info_tree);
        let mod_name = info_tree
            .get_child("Name")
            .ok_or_else(|| Error::Missing("Name".into()))?;
        let config_tree = xmltree::Element::parse(config)?;
        Ok(Fomod {})
    }
}

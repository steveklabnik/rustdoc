//! Code used to serialize crate data to JSON. We use a subset of the JSON-API spec.

use std::collections::HashMap;

// Sizes for the HashMaps to avoid reallocation and large HashMap sizes when we know
// the upper limit for them

// Number of possible values in the enum item::Metadata
pub const METADATA_SIZE: usize = 14;

// Under each item in data.relationships how many fields are there
// For now we only have one which is data.
const DATA_SIZE: usize = 1;

#[derive(Serialize, Debug)]
pub struct Documentation<'a> {
    data: Option<Document<'a>>,
    included: Option<Vec<Document<'a>>>,
}

#[derive(Serialize, Debug)]
pub struct Document<'a> {
    #[serde(rename = "type")]
    ty: &'a str,
    id: &'a str,
    attributes: HashMap<&'a str, &'a str>,
    relationships: Option<HashMap<&'a str, HashMap<&'a str, Vec<Data<'a>>>>>,
}

#[derive(Serialize, Debug)]
pub struct Data<'a> {
    #[serde(rename = "type")]
    ty: &'a str,
    id: &'a str,
}

impl<'a> Documentation<'a> {
    pub fn new() -> Self {
        Self {
            data: None,
            included: None,
        }
    }

    pub fn data(mut self, data: Document<'a>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn included(mut self, included: Vec<Document<'a>>) -> Self {
        self.included = Some(included);
        self
    }
}

impl<'a> Document<'a> {
    pub fn new() -> Self {
        Self {
            ty: "",
            id: "",
            attributes: HashMap::new(),
            relationships: None,
        }
    }

    pub fn ty(mut self, t: &'a str) -> Self {
        self.ty = t;
        self
    }

    pub fn id(mut self, id: &'a str) -> Self {
        self.id = id;
        self
    }

    pub fn attributes(mut self, attribute: &'a str, value: &'a str) -> Self {
        self.attributes.insert(attribute, value);
        self
    }

    pub fn relationships(&mut self, ty: &'a str, data: Vec<Data<'a>>) {
        // The data_map stuff won't work if things under modules contains
        // multiple different things
        match self.relationships {
            None => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert("data", data);

                let mut relationships = HashMap::with_capacity(METADATA_SIZE);
                relationships.insert(ty, data_map);
                self.relationships = Some(relationships);
            }
            Some(ref mut relationships) => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert("data", data);

                relationships.insert(ty, data_map);
            }
        }
    }
}

impl<'a> Data<'a> {
    pub fn new() -> Self {
        Self { ty: "", id: "" }
    }

    pub fn ty(mut self, t: &'a str) -> Self {
        self.ty = t;
        self
    }

    pub fn id(mut self, id: &'a str) -> Self {
        self.id = id;
        self
    }
}

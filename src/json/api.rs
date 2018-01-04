//! JSON-API types and functions. We use a subset of the JSON-API specification.

use std::collections::HashMap;

// Sizes for the HashMaps to avoid reallocation and large HashMap sizes when we know
// the upper limit for them

/// Number of possible values in the enum `analysis::DefKind`
pub const METADATA_SIZE: usize = 14;

/// Under each item in data.relationships how many fields are there. For now we only have one which
/// is data. As we might add more this number should increase
const DATA_SIZE: usize = 1;

/// The data structure used to serialize crate documentation data to JSON for consumption by the
/// front end. When fully serialized the data looks something like:
///
/// ```json
/// {
///     "data": {
///         "type": "crate",
///         "id": "example",
///         "attributes": {
///             "docs": "docs go here"
///         },
///         "relationships": {
///             "modules": {
///                 "data": [
///                     {
///                         "type": "module",
///                         "id": "example::example_module"
///                     }
///                 ]
///             }
///         }
///     },
///     "included": [
///         {
///             "type": "module",
///             "id": "example::example_module",
///             "attributes": {
///                 "docs": "docs go here",
///                 "name": "example_module"
///             }
///         }
///     ]
/// }
/// ```
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Documentation {
    /// The top level crate information and documentation
    pub data: Option<Document>,

    /// The rest of the crate information and documentation
    pub included: Option<Vec<Document>>,
}

/// A sub type of the `Documentation` struct. It contains the majority of the data. It can be used
/// for both the `data` and `included` field in the serialized JSON.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Document {
    #[serde(rename = "type")]
    /// The type of the item (e.g. "crate", "function", "enum", etc.)
    ty: String,

    /// The unique identifier associated with this item
    pub id: String,

    /// The attributes associated with the item, like documentation or its name
    pub attributes: HashMap<String, String>,

    /// An optional field used to show the relationship between the crate to the other items in the
    /// crate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<HashMap<String, HashMap<String, VecOrData>>>,
}

/// Relationships can be singular or plural, so this type makes that happen
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VecOrData {
    /// for a plural relationship
    Vec(Vec<Data>),
    /// for a singular relationship
    Data(Data),
}

/// Used to populate the `relationships` `data` field in the serialized JSON
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Data {
    #[serde(rename = "type")]
    /// The type of the item (e.g. "crate", "function", "enum", etc.)
    ty: String,

    /// The unique identifier associated with this item
    id: String,
}

impl Documentation {
    /// Create an empty `Documentation` struct
    pub fn new() -> Self {
        Self {
            data: None,
            included: None,
        }
    }

    /// Set the `data` field of a `Documentation` struct to the value passed into the
    /// `data` argument
    pub fn data(mut self, data: Document) -> Self {
        self.data = Some(data);
        self
    }

    /// Set the `included` field of a `Documentation` struct to the value passed into the
    /// `included` argument
    pub fn included(mut self, included: Vec<Document>) -> Self {
        self.included = Some(included);
        self
    }
}

impl Document {
    /// Create an empty `Document` struct
    pub fn new() -> Self {
        Self {
            ty: String::new(),
            id: String::new(),
            attributes: HashMap::new(),
            relationships: None,
        }
    }

    /// Set the `ty` field of a `Document` struct to the value passed into the
    /// `t` argument
    pub fn ty(mut self, t: String) -> Self {
        self.ty = t;
        self
    }

    /// Set the `id` field of a `Document` struct to the value passed into the
    /// `id` argument
    pub fn id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    /// Insert an attribute for the `attribute` field of a `Document` struct. If the current
    /// `attribute` exists it'll be overwritten with the given value, otherwise it'll just be
    /// created for the first time.
    pub fn attributes(mut self, attribute: String, value: String) -> Self {
        self.attributes.insert(attribute, value);
        self
    }

    /// Insert a relationship for the `relationships` field of a `Document` struct. If the current
    /// `ty` exists it'll be overwritten with the given `data` value, otherwise it'll just be
    /// created for the first time.
    pub fn relationships(&mut self, ty: String, data: Vec<Data>) {
        match self.relationships {
            // The code below allows us to handle with ease the fact we have data structured like:
            //
            // "relationships": {
            //     "modules": {
            //         "data": [
            //             {
            //                 "type": "module",
            //                 "id": "example::example_module"
            //             }
            //         ]
            //     }
            //     "type": {
            //         "data": [
            //             {
            //                 "type": "type",
            //                 "id": "example::example_type"
            //             }
            //         ]
            //     }
            // }
            //
            // Where they all have the data field because of JSON API. However, having others
            // handle this with the builder pattern having three fields can be cumbersome. Unless
            // more fields like "data" are added this method handles it automatically.

            // If the `relationships` field does not exist, add the new type relationship (e.g.
            // module, fn, type, etc.) and the data to a newly created relationship map and assign
            // it to the `Document`.
            None => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert(String::from("data"), VecOrData::Vec(data));

                let mut relationships = HashMap::with_capacity(METADATA_SIZE);
                relationships.insert(ty, data_map);
                self.relationships = Some(relationships);
            }
            // If the `relationships` field exists, add the new type relationship (e.g. module, fn,
            // type, etc.) and the data.
            Some(ref mut relationships) => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert(String::from("data"), VecOrData::Vec(data));

                relationships.insert(ty, data_map);
            }
        }
    }

    /// Like relationships, but if the key already exists, appends data rather than
    /// overwriting.
    pub fn add_relationship(&mut self, ty: String, data: Data) {
        match self.relationships {
            None => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert(String::from("data"), VecOrData::Vec(vec![data]));

                let mut relationships = HashMap::with_capacity(METADATA_SIZE);
                relationships.insert(ty, data_map);
                self.relationships = Some(relationships);
            }
            Some(ref mut relationships) => {
                let entry = relationships.entry(ty).or_insert_with(|| {
                    let mut map = HashMap::with_capacity(DATA_SIZE);
                    map.insert(String::from("data"), VecOrData::Vec(Vec::new()));
                    map
                });

                // We know that we have a "data" entry because we created it above
                // and we know it's a Vec for the same reason
                if let &mut VecOrData::Vec(ref mut data_vec) = entry.get_mut("data").unwrap() {
                    data_vec.push(data);
                }
            }
        }
    }

    /// Like add_relationship, but makes it singular
    ///
    /// if the relationship already exists, nothing happens
    pub fn add_singular_relationship(&mut self, ty: String, data: Data) {
        match self.relationships {
            None => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert(String::from("data"), VecOrData::Data(data));

                let mut relationships = HashMap::with_capacity(METADATA_SIZE);
                relationships.insert(ty, data_map);
                self.relationships = Some(relationships);
            }
            Some(ref mut relationships) => {
                relationships.entry(ty).or_insert_with(|| {
                    let mut map = HashMap::with_capacity(DATA_SIZE);
                    map.insert(String::from("data"), VecOrData::Data(data));
                    map
                });
            }
        }
    }
}

impl Data {
    /// Create a new empty `Data` struct
    pub fn new() -> Self {
        Self {
            ty: String::new(),
            id: String::new(),
        }
    }

    /// Set the `ty` field of `Data` to the value passed to the `t` argument
    pub fn ty(mut self, t: String) -> Self {
        self.ty = t;
        self
    }

    /// Set the `id` field of `Data` to the value passed to the `id` argument
    pub fn id(mut self, id: String) -> Self {
        self.id = id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Data, Document, Documentation, VecOrData};

    use serde_json;

    #[test]
    fn relationships() {
        let mut document = Document::new();
        assert!(document.relationships.is_none());

        document.relationships(
            "modules".into(),
            vec![
                Data::new()
                    .ty("module".into())
                    .id("example::module_one".into()),
            ],
        );

        {
            let relationships = &document.relationships.as_ref().unwrap()["modules"]["data"];
            if let &VecOrData::Vec(ref relationships) = relationships {
                assert_eq!(relationships.len(), 1);
                assert_eq!(&relationships[0].id, "example::module_one");
            } else {
                panic!("relationship was not plural");
            }
        }

        document.relationships(
            "modules".into(),
            vec![
                Data::new()
                    .ty("module".into())
                    .id("example::module_two".into()),
            ],
        );

        {
            let relationships = &document.relationships.as_ref().unwrap()["modules"]["data"];
            if let &VecOrData::Vec(ref relationships) = relationships {
                assert_eq!(relationships.len(), 1);
                assert_eq!(&relationships[0].id, "example::module_two");
            } else {
                panic!("relationship was not plural");
            }
        }
    }

    #[test]
    fn serialize() {
        let module_data = Data::new().ty("module".into()).id("example::module".into());
        let module_data_json =
            json!({});
        assert_eq!(
            module_data_json,
            serde_json::to_value(&module_data).unwrap()
        );

        let mut krate = Document::new()
            .ty("crate".into())
            .id("example".into())
            .attributes("docs".into(), "crate docs".into());

        let module = Document::new()
            .ty("module".into())
            .id("example::module".into())
            .attributes("docs".into(), "module docs".into())
            .attributes("name".into(), "module".into());
        let module_json =
            json!({});
        assert_eq!(serde_json::to_value(&module).unwrap(), module_json);

        krate.relationships("modules".into(), vec![module_data]);

        let documentation = Documentation::new().data(krate).included(vec![module]);
        assert_eq!(
            serde_json::to_value(&documentation).unwrap(),
            json!({
                "data": {
                    "type": "crate",
                    "id": "example",
                    "attributes": {
                        "docs": "crate docs",
                    },
                    "relationships": {
                        "modules": {
                            "data": [ module_data_json ]
                        }
                    }
                },
                "included": [ module_json ],
            })
        );
    }
}

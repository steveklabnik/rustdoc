//! Code used to serialize crate data to JSON. We use a subset of the JSON-API spec.

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
#[derive(Debug, Default, Serialize)]
pub struct Documentation {
    /// The top level crate information and documentation
    data: Option<Document>,

    /// The rest of the crate information and documentation
    included: Option<Vec<Document>>,
}

/// A sub type of the `Documentation` struct. It contains the majority of the data. It can be used
/// for both the `data` and `included` field in the serialized JSON.
#[derive(Debug, Default, Serialize)]
pub struct Document {
    #[serde(rename = "type")]
    /// The type of the item (e.g. "crate", "function", "enum", etc.)
    ty: String,

    /// The unique identifier associated with this item
    id: String,

    /// The attributes associated with the item, like documentation or its name
    attributes: HashMap<String, String>,

    /// An optional field used to show the relationship between the crate to the other items in the
    /// crate
    relationships: Option<HashMap<String, HashMap<String, Vec<Data>>>>,
}

/// Used to populate the `relationships` `data` field in the serialized JSON
#[derive(Debug, Default, Serialize)]
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
                data_map.insert(String::from("data"), data);

                let mut relationships = HashMap::with_capacity(METADATA_SIZE);
                relationships.insert(ty, data_map);
                self.relationships = Some(relationships);
            }
            // If the `relationships` field exists, add the new type relationship (e.g. module, fn,
            // type, etc.) and the data.
            Some(ref mut relationships) => {
                let mut data_map = HashMap::with_capacity(DATA_SIZE);
                data_map.insert(String::from("data"), data);

                relationships.insert(ty, data_map);
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
    use super::{Data, Document, Documentation};

    use serde_json;

    #[test]
    fn serialize() {
        let module_data = Data::new().ty("module".into()).id("example::module".into());
        let module_data_json = json!({
            "type": "module",
            "id": "example::module",
        });
        assert_eq!(module_data_json, serde_json::to_value(&module_data).unwrap());

        let mut krate = Document::new()
            .ty("crate".into())
            .id("example".into())
            .attributes("docs".into(), "crate docs".into());

        let module = Document::new()
            .ty("module".into())
            .id("example::module".into())
            .attributes("docs".into(), "module docs".into())
            .attributes("name".into(), "module".into());
        let module_json = json!({
            "type": "module",
            "id": "example::module",
            "attributes": {
                "docs": "module docs",
                "name": "module",
            },
            "relationships": null,
        });
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

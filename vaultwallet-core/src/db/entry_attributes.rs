use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryAttributes {
	pub strings: HashMap<String, String>,
}

impl EntryAttributes {
	pub fn get(&self, key: &str) -> Option<&str> {
		self.strings.get(key).map(|s| s.as_str())
	}
}

use crate::db::entry_attributes::EntryAttributes;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
	pub uuid: Uuid,
	pub attrs: EntryAttributes,
}

impl Entry {
	pub fn new(uuid: Uuid) -> Self {
		Self {
			uuid,
			attrs: EntryAttributes::default(),
		}
	}

	pub fn title(&self) -> Option<&str> {
		self.attrs.get("Title")
	}

	pub fn username(&self) -> Option<&str> {
		self.attrs.get("UserName")
	}

	pub fn password(&self) -> Option<&str> {
		self.attrs.get("Password")
	}

	pub fn url(&self) -> Option<&str> {
		self.attrs.get("URL")
	}

	pub fn get_field(&self, name: &str) -> Option<&str> {
		self.attrs.get(name)
	}
}

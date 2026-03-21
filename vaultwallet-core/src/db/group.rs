use crate::db::entry::Entry;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
	pub uuid: Uuid,
	pub name: String,
	pub entries: Vec<Entry>,
	pub children: Vec<Group>,
}

impl Group {
	pub fn new(uuid: Uuid) -> Self {
		Self {
			uuid,
			name: String::new(),
			entries: Vec::new(),
			children: Vec::new(),
		}
	}

	pub fn entries(&self) -> impl Iterator<Item = &Entry> {
		self.entries.iter()
	}

	pub fn subgroups(&self) -> impl Iterator<Item = &Group> {
		self.children.iter()
	}
}

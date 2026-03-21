use crate::db::group::Group;
use crate::db::metadata::Metadata;
use crate::settings::VaultSettings;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Database {
	pub metadata: Metadata,
	pub root: Group,
	pub settings: VaultSettings,
}

impl Database {
	pub fn root_group(&self) -> &Group {
		&self.root
	}
}

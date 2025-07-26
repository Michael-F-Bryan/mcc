use codespan_reporting::files::{Error, SimpleFile};

use crate::{Db, Text, types::SourceFile};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Files(Vec<FileEntry>);

impl Files {
    pub const fn new() -> Self {
        Files(Vec::new())
    }

    fn get(&self, id: SourceFile) -> Result<&FileEntry, Error> {
        self.0
            .iter()
            .find(|f| f.key == id)
            .ok_or(Error::FileMissing)
    }

    pub fn add(&mut self, db: &dyn Db, file: SourceFile) {
        let new_entry = FileEntry {
            key: file,
            file: SimpleFile::new(file.path(db).clone(), file.contents(db).clone()),
        };

        if self.0.iter().any(|f| f.key == file) {
            // Already exists - no need to add it again
        } else {
            self.0.push(new_entry);
        }
    }
}

impl codespan_reporting::files::Files<'_> for Files {
    type FileId = SourceFile;
    type Name = Text;
    type Source = Text;

    fn name(&'_ self, id: Self::FileId) -> Result<Self::Name, Error> {
        let entry = self.get(id)?;
        Ok(entry.file.name().clone())
    }

    fn source(&'_ self, id: Self::FileId) -> Result<Self::Source, Error> {
        let entry = self.get(id)?;
        Ok(entry.file.source().clone())
    }

    fn line_index(&'_ self, id: Self::FileId, byte_index: usize) -> Result<usize, Error> {
        self.get(id)?.file.line_index((), byte_index)
    }

    fn line_range(
        &'_ self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<std::ops::Range<usize>, Error> {
        self.get(id)?.file.line_range((), line_index)
    }
}

#[derive(Debug, Clone)]
struct FileEntry {
    key: SourceFile,
    file: SimpleFile<Text, Text>,
}

impl PartialEq for FileEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.file.name() == other.file.name()
            && self.file.source() == other.file.source()
    }
}

impl Eq for FileEntry {}

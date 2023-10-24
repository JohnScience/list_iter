#![allow(dead_code, unused_imports)]

use stack_trait::Stack;
use std::{collections::HashMap, ops::Deref};
use vec_vec::{TrivialLastEntry, VecVecExt};

struct MockFtpStream(HashMap<usize, Vec<MockDirEntry>>);

impl MockFtpStream {
    // mocks the `list` command
    fn list(&mut self, id: usize) -> Option<Vec<MockDirEntry>> {
        if self.0.is_empty() {
            return None;
        }
        if id == 0 {
            Some(self.0.get(&0).unwrap().clone())
        } else {
            self.0.get(&id).map(Clone::clone)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MockDirEntry {
    NonDir { fd: usize },
    Dir { fd: usize },
}

impl MockDirEntry {
    fn fd(&self) -> usize {
        match self {
            MockDirEntry::NonDir { fd } => *fd,
            MockDirEntry::Dir { fd, .. } => *fd,
        }
    }
}

struct MockDFSListIter<'a> {
    ftp_stream: &'a mut MockFtpStream,
    list_results: Vec<Vec<MockDirEntry>>,
    prevent_dive: bool,
}

impl<'a> MockDFSListIter<'a> {
    fn new(ftp_stream: &'a mut MockFtpStream) -> Self {
        const ROOT: usize = 0;
        let root_list = ftp_stream.list(ROOT).unwrap();
        MockDFSListIter {
            ftp_stream,
            list_results: vec![root_list],
            prevent_dive: false,
        }
    }

    fn entry(
        list_results: &mut Vec<Vec<MockDirEntry>>,
    ) -> Option<TrivialLastEntry<'_, MockDirEntry>> {
        while list_results.last()?.is_empty() {
            list_results.pop();
        }
        // ? will never return None
        list_results.trivial_last_entry()
    }
}

impl<'a> Iterator for MockDFSListIter<'a> {
    type Item = MockDirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let Self {
            ftp_stream,
            list_results,
            prevent_dive,
        } = self;
        let mut entry = Self::entry(list_results)?;
        loop {
            match &*entry {
                MockDirEntry::NonDir { .. } => {
                    *prevent_dive = entry.is_last_in_inner();
                    return Some(entry.pop_pointee());
                }
                MockDirEntry::Dir { fd } => {
                    let list = ftp_stream.list(*fd).unwrap();
                    if list.is_empty() || *prevent_dive {
                        *prevent_dive = entry.is_last_in_inner();
                        return Some(entry.pop_pointee());
                    }
                    *prevent_dive = false;
                    entry.push_to_outer(list);
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn empty_entries() {
        let mut ftp_stream = MockFtpStream(HashMap::new());
        // this will panic because the root directory is empty
        let mut list_iter = MockDFSListIter::new(&mut ftp_stream);
        assert_eq!(list_iter.next(), None);
    }

    #[test]
    fn empty_root() {
        let mut dir_map = HashMap::new();
        dir_map.insert(0, vec![]);
        let mut ftp_stream = MockFtpStream(dir_map);
        let mut list_iter = MockDFSListIter::new(&mut ftp_stream);
        assert_eq!(list_iter.next(), None);
    }

    #[test]
    fn one_file() {
        let mut dir_map = HashMap::new();
        dir_map.insert(0, vec![MockDirEntry::NonDir { fd: 1 }]);
        let mut ftp_stream = MockFtpStream(dir_map);
        let mut list_iter = MockDFSListIter::new(&mut ftp_stream);
        assert_eq!(list_iter.next(), Some(MockDirEntry::NonDir { fd: 1 }));
        assert_eq!(list_iter.next(), None);
    }

    #[test]
    fn three_files() {
        let mut dir_map = HashMap::new();
        dir_map.insert(
            0,
            vec![
                MockDirEntry::NonDir { fd: 1 },
                MockDirEntry::NonDir { fd: 2 },
                MockDirEntry::NonDir { fd: 3 },
            ],
        );
        let mut ftp_stream = MockFtpStream(dir_map);
        let mut list_iter = MockDFSListIter::new(&mut ftp_stream);
        // They are received in reverse order because the last entry is popped first.
        assert_eq!(list_iter.next(), Some(MockDirEntry::NonDir { fd: 3 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::NonDir { fd: 2 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::NonDir { fd: 1 }));

        assert_eq!(list_iter.next(), None);
    }

    #[test]
    fn empty_dir_in_root() {
        let mut dir_map = HashMap::new();
        dir_map.insert(0, vec![MockDirEntry::Dir { fd: 1 }]);
        dir_map.insert(1, vec![]);
        let mut ftp_stream = MockFtpStream(dir_map);
        let mut list_iter = MockDFSListIter::new(&mut ftp_stream);
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 1 }));
        assert_eq!(list_iter.next(), None);
    }

    #[test]
    fn two_nested_dirs_in_root() {
        let mut dir_map = HashMap::new();
        dir_map.insert(0, vec![MockDirEntry::Dir { fd: 1 }]);
        dir_map.insert(1, vec![MockDirEntry::Dir { fd: 2 }]);
        dir_map.insert(2, vec![]);

        let mut ftp_stream = MockFtpStream(dir_map);
        let mut list_iter = MockDFSListIter::new(&mut ftp_stream);

        // dirs are traversed depth-first
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 2 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 1 }));
        assert_eq!(list_iter.next(), None);
    }

    #[test]
    fn pair_of_nested_dirs_with_files() {
        let mut dir_map = HashMap::new();
        dir_map.insert(
            0,
            vec![MockDirEntry::Dir { fd: 1 }, MockDirEntry::Dir { fd: 4 }],
        );
        dir_map.insert(1, vec![MockDirEntry::Dir { fd: 2 }]);
        dir_map.insert(2, vec![MockDirEntry::NonDir { fd: 3 }]);
        dir_map.insert(4, vec![MockDirEntry::Dir { fd: 5 }]);
        dir_map.insert(5, vec![MockDirEntry::NonDir { fd: 6 }]);

        let mut ftp_stream = MockFtpStream(dir_map);
        let mut list_iter = MockDFSListIter::new(&mut ftp_stream);

        assert_eq!(list_iter.next(), Some(MockDirEntry::NonDir { fd: 6 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 5 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 4 }));

        assert_eq!(list_iter.next(), Some(MockDirEntry::NonDir { fd: 3 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 2 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 1 }));
        assert_eq!(list_iter.next(), None);
    }

    #[test]
    fn pair_of_nested_dirs_with_files_and_empty_dir() {
        let mut dir_map = HashMap::new();
        dir_map.insert(
            0,
            vec![
                MockDirEntry::Dir { fd: 1 },
                MockDirEntry::Dir { fd: 4 },
                MockDirEntry::Dir { fd: 7 },
            ],
        );
        dir_map.insert(1, vec![MockDirEntry::Dir { fd: 2 }]);
        dir_map.insert(2, vec![MockDirEntry::NonDir { fd: 3 }]);
        dir_map.insert(4, vec![MockDirEntry::Dir { fd: 5 }]);
        dir_map.insert(5, vec![MockDirEntry::NonDir { fd: 6 }]);
        dir_map.insert(7, vec![]);

        let mut ftp_stream = MockFtpStream(dir_map);
        let mut list_iter = MockDFSListIter::new(&mut ftp_stream);

        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 7 }));

        assert_eq!(list_iter.next(), Some(MockDirEntry::NonDir { fd: 6 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 5 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 4 }));

        assert_eq!(list_iter.next(), Some(MockDirEntry::NonDir { fd: 3 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 2 }));
        assert_eq!(list_iter.next(), Some(MockDirEntry::Dir { fd: 1 }));
        assert_eq!(list_iter.next(), None);
    }
}

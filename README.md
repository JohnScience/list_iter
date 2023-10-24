# Draft implementation of an iterator over a directory on an FTP server

At the moment of writing, [`suppaftp`] does not provide an iterator over the entries (files, dirs, symlinks) on a remote server. This crate provides a draft implementation of such an iterator. It minimizes the number of requests to the FTP server (LIST command invokations) by using a cache.

Writing the iterator proved to be a daunting task for me because I was facing the shortcomings of the pre-Polonius NLL borrow checker and because controlling "diving"/"rising" behaviour is tricky. I am not sure if the code is correct, but it seems to work for the 8 tests that I've written.

[`suppaftp`]: https://crates.io/crates/suppaftp/5.2.0

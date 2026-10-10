// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::host::Namespace;

pub(crate) const F_OK: i32 = 0;
pub(crate) const R_OK: i32 = 4;
pub(crate) const W_OK: i32 = 2;
pub(crate) const X_OK: i32 = 1;

pub(crate) const S_IFMT: i32 = 0o170000;
pub(crate) const S_IFREG: i32 = 0o100000;
pub(crate) const S_IFDIR: i32 = 0o40000;
pub(crate) const S_IFCHR: i32 = 0o20000;
pub(crate) const S_IFBLK: i32 = 0o60000;
pub(crate) const S_IFIFO: i32 = 0o10000;
pub(crate) const S_IFLNK: i32 = 0o120000;
pub(crate) const S_IFSOCK: i32 = 0o140000;

pub(crate) const S_IRWXU: i32 = 0o700;
pub(crate) const S_IRUSR: i32 = 0o400;
pub(crate) const S_IWUSR: i32 = 0o200;
pub(crate) const S_IXUSR: i32 = 0o100;
pub(crate) const S_IRWXG: i32 = 0o70;
pub(crate) const S_IRGRP: i32 = 0o40;
pub(crate) const S_IWGRP: i32 = 0o20;
pub(crate) const S_IXGRP: i32 = 0o10;
pub(crate) const S_IRWXO: i32 = 0o7;
pub(crate) const S_IROTH: i32 = 0o4;
pub(crate) const S_IWOTH: i32 = 0o2;
pub(crate) const S_IXOTH: i32 = 0o1;

pub(crate) const O_RDONLY: i32 = 0;
pub(crate) const O_WRONLY: i32 = 1;
pub(crate) const O_RDWR: i32 = 2;
pub(crate) const O_CREAT: i32 = 64;
pub(crate) const O_EXCL: i32 = 128;
pub(crate) const O_TRUNC: i32 = 512;
pub(crate) const O_APPEND: i32 = 1024;
pub(crate) const O_NOFOLLOW: i32 = 131072;

pub(crate) const COPYFILE_EXCL: i32 = 1;
pub(crate) const COPYFILE_FICLONE: i32 = 2;
pub(crate) const COPYFILE_FICLONE_FORCE: i32 = 4;

pub struct Constants;

impl Constants {
    pub fn build(namespace: &mut Namespace) {
        namespace.constant("F_OK", F_OK);
        namespace.constant("R_OK", R_OK);
        namespace.constant("W_OK", W_OK);
        namespace.constant("X_OK", X_OK);

        namespace.constant("S_IFMT", S_IFMT);
        namespace.constant("S_IFREG", S_IFREG);
        namespace.constant("S_IFDIR", S_IFDIR);
        namespace.constant("S_IFCHR", S_IFCHR);
        namespace.constant("S_IFBLK", S_IFBLK);
        namespace.constant("S_IFIFO", S_IFIFO);
        namespace.constant("S_IFLNK", S_IFLNK);
        namespace.constant("S_IFSOCK", S_IFSOCK);

        namespace.constant("S_IRWXU", S_IRWXU);
        namespace.constant("S_IRUSR", S_IRUSR);
        namespace.constant("S_IWUSR", S_IWUSR);
        namespace.constant("S_IXUSR", S_IXUSR);
        namespace.constant("S_IRWXG", S_IRWXG);
        namespace.constant("S_IRGRP", S_IRGRP);
        namespace.constant("S_IWGRP", S_IWGRP);
        namespace.constant("S_IXGRP", S_IXGRP);
        namespace.constant("S_IRWXO", S_IRWXO);
        namespace.constant("S_IROTH", S_IROTH);
        namespace.constant("S_IWOTH", S_IWOTH);
        namespace.constant("S_IXOTH", S_IXOTH);

        namespace.constant("O_RDONLY", O_RDONLY);
        namespace.constant("O_WRONLY", O_WRONLY);
        namespace.constant("O_RDWR", O_RDWR);
        namespace.constant("O_CREAT", O_CREAT);
        namespace.constant("O_EXCL", O_EXCL);
        namespace.constant("O_TRUNC", O_TRUNC);
        namespace.constant("O_APPEND", O_APPEND);
        namespace.constant("O_NOFOLLOW", O_NOFOLLOW);

        namespace.constant("COPYFILE_EXCL", COPYFILE_EXCL);
        namespace.constant("COPYFILE_FICLONE", COPYFILE_FICLONE);
        namespace.constant("COPYFILE_FICLONE_FORCE", COPYFILE_FICLONE_FORCE);
    }
}

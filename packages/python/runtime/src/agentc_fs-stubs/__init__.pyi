from collections.abc import AsyncIterator
from datetime import datetime
from os import PathLike
from types import TracebackType
from typing import Literal, Self, final

type StrPath = str | PathLike[str]
type EntryType = Literal[
    "file",
    "directory",
    "symlink",
    "fifo",
    "socket",
    "block_device",
    "character_device",
    "other",
]


@final
class Stat:
    """
    Metadata for a filesystem entry.
    """

    @property
    def type(self) -> EntryType:
        """
        The entry type.
        """
    @property
    def is_file(self) -> bool:
        """
        Whether the entry is a regular file.
        """
    @property
    def is_dir(self) -> bool:
        """
        Whether the entry is a directory.
        """
    @property
    def is_symlink(self) -> bool:
        """
        Whether the entry is a symbolic link.
        """
    @property
    def size(self) -> int:
        """
        The entry size in bytes.
        """
    @property
    def mode(self) -> int:
        """
        The permission mode.
        """
    @property
    def is_readonly(self) -> bool:
        """
        Whether the entry is read-only.
        """
    @property
    def accessed(self) -> datetime | None:
        """
        The last access time, if available.
        """
    @property
    def modified(self) -> datetime | None:
        """
        The last modification time, if available.
        """
    @property
    def created(self) -> datetime | None:
        """
        The creation time, if available.
        """
    @property
    def changed(self) -> datetime | None:
        """
        The last metadata change time, if available.
        """
    @property
    def dev(self) -> int:
        """
        The device identifier.
        """
    @property
    def ino(self) -> int:
        """
        The inode number.
        """
    @property
    def nlink(self) -> int:
        """
        The number of hard links.
        """
    @property
    def uid(self) -> int:
        """
        The owner user identifier.
        """
    @property
    def gid(self) -> int:
        """
        The owner group identifier.
        """
    @property
    def rdev(self) -> int:
        """
        The device identifier for a special file.
        """
    @property
    def blksize(self) -> int:
        """
        The preferred block size for I/O.
        """
    @property
    def blocks(self) -> int:
        """
        The number of allocated 512-byte blocks.
        """


@final
class Entry:
    """
    An entry returned by directory iteration.
    """

    @property
    def name(self) -> str:
        """
        The entry name.
        """
    @property
    def path(self) -> str:
        """
        The entry path.
        """
    @property
    def type(self) -> EntryType:
        """
        The entry type.
        """
    @property
    def is_file(self) -> bool:
        """
        Whether the entry is a regular file.
        """
    @property
    def is_dir(self) -> bool:
        """
        Whether the entry is a directory.
        """
    @property
    def is_symlink(self) -> bool:
        """
        Whether the entry is a symbolic link.
        """
    @property
    def stat(self) -> Stat | None:
        """
        The entry metadata, if available.
        """


@final
class File:
    """
    An open filesystem file.
    """

    @property
    def closed(self) -> bool:
        """
        Whether the file is closed.
        """
    async def read(self, size: int = -1, /) -> bytes:
        """
        Read bytes from the current position.
        """
    async def read_text(self) -> str:
        """
        Read text from the current position.
        """
    async def write(self, data: bytes, /) -> None:
        """
        Write bytes at the current position.
        """
    async def seek(self, offset: int, whence: int = 0, /) -> int:
        """
        Move the current position and return its new value.
        """
    async def tell(self) -> int:
        """
        Return the current position.
        """
    async def truncate(self, size: int, /) -> None:
        """
        Resize the file.
        """
    async def flush(self) -> None:
        """
        Flush buffered writes.
        """
    async def fsync(self) -> None:
        """
        Synchronize file data and metadata.
        """
    async def fdatasync(self) -> None:
        """
        Synchronize file data.
        """
    async def close(self) -> None:
        """
        Close the file.
        """
    async def __aenter__(self) -> Self:
        """
        Enter the asynchronous context manager.
        """
    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        """
        Close the file when leaving the asynchronous context manager.
        """


@final
class Directory:
    """
    A directory within a filesystem authority.
    """

    @staticmethod
    def root() -> Directory:
        """
        Return the configured root directory.
        """
    @property
    def path(self) -> str:
        """
        The directory path.
        """
    @property
    def authority_root(self) -> str:
        """
        The root path of the directory authority.
        """

    async def open(
        self,
        path: StrPath,
        /,
        *,
        read: bool = True,
        write: bool = False,
        append: bool = False,
        truncate: bool = False,
        create: bool = False,
        create_new: bool = False,
        follow_symlinks: bool = True,
    ) -> File:
        """
        Open a file with the requested options.
        """
    async def read_bytes(self, path: StrPath, /) -> bytes:
        """
        Read a file as bytes.
        """
    async def read_text(self, path: StrPath, /) -> str:
        """
        Read a file as text.
        """
    async def write_bytes(self, path: StrPath, data: bytes, /) -> None:
        """
        Write bytes to a file.
        """
    async def write_text(self, path: StrPath, data: str, /) -> None:
        """
        Write text to a file.
        """
    async def truncate(self, path: StrPath, length: int, /) -> None:
        """
        Resize a file.
        """

    async def open_dir(self, path: StrPath, /) -> Directory:
        """
        Open a child directory.
        """
    async def mkdir(
        self,
        path: StrPath,
        /,
        *,
        parents: bool = False,
        exist_ok: bool = False,
    ) -> Directory:
        """
        Create and return a child directory.
        """
    async def mkdtemp(self, *, prefix: StrPath) -> Directory:
        """
        Create and return a uniquely named temporary directory.
        """
    def scandir(self) -> AsyncIterator[Entry]:
        """
        Iterate over the direct children of this directory.
        """
    def walk(self) -> AsyncIterator[Entry]:
        """
        Iterate recursively over descendants of this directory.
        """
    async def entry(self, path: StrPath, /) -> Entry:
        """
        Return a directory entry.
        """

    async def stat(self, path: StrPath, /, *, follow_symlinks: bool = True) -> Stat:
        """
        Return metadata for an entry.
        """
    async def exists(self, path: StrPath, /, *, follow_symlinks: bool = True) -> bool:
        """
        Return whether an entry exists.
        """
    async def access(
        self,
        path: StrPath,
        /,
        *,
        read: bool = False,
        write: bool = False,
        execute: bool = False,
        follow_symlinks: bool = True,
    ) -> bool:
        """
        Check the requested access permissions for an entry.
        """
    async def chmod(self, path: StrPath, mode: int, /) -> None:
        """
        Change an entry's permission mode.
        """
    async def chown(
        self,
        path: StrPath,
        /,
        *,
        uid: int | None = None,
        gid: int | None = None,
        follow_symlinks: bool = True,
    ) -> None:
        """
        Change an entry's owner or group.
        """

    async def symlink(self, target: StrPath, link: StrPath, /) -> None:
        """
        Create a symbolic link.
        """
    async def readlink(self, path: StrPath, /) -> str:
        """
        Return the target of a symbolic link.
        """
    async def rename(self, source: StrPath, destination: StrPath, /) -> None:
        """
        Rename or move an entry.
        """
    async def remove(self, path: StrPath, /) -> None:
        """
        Remove a file or symbolic link.
        """
    async def rmdir(self, path: StrPath, /) -> None:
        """
        Remove an empty directory.
        """
    async def rmtree(self, path: StrPath, /) -> None:
        """
        Remove a directory and its contents recursively.
        """


class FilesystemError(Exception):
    """
    Base class for filesystem errors.
    """

    @property
    def message(self) -> str:
        """
        The error message.
        """

class NotFoundError(FilesystemError):
    """
    An error raised when an entry does not exist.
    """

    @property
    def path(self) -> str:
        """
        The path that was not found.
        """

class AlreadyExistsError(FilesystemError):
    """
    An error raised when an entry already exists.
    """

    @property
    def path(self) -> str:
        """
        The path that already exists.
        """

class NotDirectoryError(FilesystemError):
    """
    An error raised when an entry is not a directory.
    """

    @property
    def path(self) -> str:
        """
        The path that is not a directory.
        """

class IsDirectoryError(FilesystemError):
    """
    An error raised when an entry is a directory.
    """

    @property
    def path(self) -> str:
        """
        The path that is a directory.
        """

class PermissionDeniedError(FilesystemError):
    """
    An error raised when access to an entry is denied.
    """

    @property
    def path(self) -> str:
        """
        The path for which access was denied.
        """

class PathEscapesAuthorityError(FilesystemError):
    """
    An error raised when a path escapes its filesystem authority.
    """

    @property
    def path(self) -> str:
        """
        The path that escapes the authority.
        """

class CrossBackendRenameError(FilesystemError):
    """
    An error raised when a rename crosses filesystem backends.
    """

    @property
    def source(self) -> str:
        """
        The source path.
        """
    @property
    def destination(self) -> str:
        """
        The destination path.
        """

class InvalidPathError(FilesystemError):
    """
    An error raised for an invalid path.
    """

class UnsupportedError(FilesystemError):
    """
    An error raised for an unsupported operation.
    """

class UnexpectedError(FilesystemError):
    """
    An error raised for an unexpected filesystem failure.
    """

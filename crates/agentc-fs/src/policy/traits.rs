// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use crate::policy::context::{
    AccessContext, EntriesContext, MetadataContext, OpenContext, RemoveContext, RenameContext,
    SymlinkContext, WriteContext,
};

pub struct Denied {
    reason: Cow<'static, str>,
}

impl Denied {
    pub fn new(reason: impl Into<Cow<'static, str>>) -> Self {
        Denied { reason: reason.into() }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

pub trait Policy: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn check_access(&self, _context: &AccessContext<'_>) -> Result<(), Denied> {
        Ok(())
    }

    fn check_open(&self, _context: &OpenContext<'_>) -> Result<(), Denied> {
        Ok(())
    }

    fn check_entries(&self, _context: &EntriesContext<'_>) -> Result<(), Denied> {
        Ok(())
    }

    fn check_metadata(&self, _context: &MetadataContext<'_>) -> Result<(), Denied> {
        Ok(())
    }

    fn check_write(&self, _context: &WriteContext<'_>) -> Result<(), Denied> {
        Ok(())
    }

    fn check_remove(&self, _context: &RemoveContext<'_>) -> Result<(), Denied> {
        Ok(())
    }

    fn check_rename(&self, _context: &RenameContext<'_>) -> Result<(), Denied> {
        Ok(())
    }

    fn check_symlink(&self, _context: &SymlinkContext<'_>) -> Result<(), Denied> {
        Ok(())
    }
}

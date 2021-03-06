// RGB standard library
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use crate::api::{self, Multipart};
use crate::error::ApiErrorType;
use core::convert::TryFrom;

#[derive(Clone, PartialEq, Debug, Display)]
#[display_from(Debug)]
#[non_exhaustive]
pub enum Request {
    Issue(api::fungible::Issue),
    Transfer(api::fungible::Transfer),
}

impl TryFrom<Multipart> for Request {
    type Error = ApiErrorType;

    fn try_from(value: Multipart) -> Result<Self, Self::Error> {
        unimplemented!()
    }
}

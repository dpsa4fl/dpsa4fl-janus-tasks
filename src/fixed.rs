
use fixed::types::extra::{U15, U31, U63};
use fixed::{FixedI16, FixedI32, FixedI64};

use downcast_rs::DowncastSync;
use dyn_clone::DynClone;
use serde::{Serialize, Deserialize};

// To create a trait with downcasting methods, extend `Downcast` or `DowncastSync`
// and run `impl_downcast!()` on the trait.
pub trait FixedBase: DowncastSync + DynClone {}
impl_downcast!(sync FixedBase);  // `sync` => also produce `Arc` downcasts.

// implements `Clone` for FixedBase, based on the `DynClone` super trait
dyn_clone::clone_trait_object!(FixedBase);


///////////////////////////////////////////////////
// Type names

pub type Fixed16 = FixedI16<U15>;
pub type Fixed32 = FixedI32<U31>;
pub type Fixed64 = FixedI64<U63>;

pub type FixedDyn = Box<dyn FixedBase>;

pub type NoiseParameterType = FixedDyn;

///////////////////////////////////////////////////
// casting
impl FixedBase for Fixed16 {}
impl FixedBase for Fixed32 {}
impl FixedBase for Fixed64 {}


///////////////////////////////////////////////////
// Type dispatch

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedTypeTag
{
    FixedType16Bit,
    FixedType32Bit,
    FixedType64Bit,
}


pub trait IsTagInstance<Tag>
{
    fn get_tag() -> Tag;
}

// instances
impl IsTagInstance<FixedTypeTag> for Fixed16
{
    fn get_tag() -> FixedTypeTag {
        FixedTypeTag::FixedType16Bit
    }
}
impl IsTagInstance<FixedTypeTag> for Fixed32
{
    fn get_tag() -> FixedTypeTag {
        FixedTypeTag::FixedType32Bit
    }
}
impl IsTagInstance<FixedTypeTag> for Fixed64
{
    fn get_tag() -> FixedTypeTag {
        FixedTypeTag::FixedType64Bit
    }
}

//////////////////////////////////////////////////
// Custom dynamic

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum FixedAny
{
    Fixed16(Fixed16),
    Fixed32(Fixed32),
    Fixed64(Fixed64),
}

impl FixedAny
{
    pub fn get_tag(&self) -> FixedTypeTag
    {
        match self
        {
            FixedAny::Fixed16(_) => FixedTypeTag::FixedType16Bit,
            FixedAny::Fixed32(_) => FixedTypeTag::FixedType32Bit,
            FixedAny::Fixed64(_) => FixedTypeTag::FixedType64Bit,
        }
    }

    pub fn downcast<X : IsTagInstance<FixedTypeTag> + FixedBase>(self) -> Option<X>
    {
        match (X::get_tag(), self)
        {
            (FixedTypeTag::FixedType16Bit, FixedAny::Fixed16(a)) => {
                let a : FixedDyn = Box::new(a);
                a.downcast::<X>().ok().map(|x| *x)
            },
            (FixedTypeTag::FixedType32Bit, FixedAny::Fixed32(a)) => {
                let a : FixedDyn = Box::new(a);
                a.downcast::<X>().ok().map(|x| *x)
            },
            (FixedTypeTag::FixedType64Bit, FixedAny::Fixed64(a)) => {
                let a : FixedDyn = Box::new(a);
                a.downcast::<X>().ok().map(|x| *x)
            },
            (_,_) => None,
        }
    }
}









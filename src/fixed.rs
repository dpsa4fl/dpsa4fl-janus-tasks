
use fixed::types::extra::{U15, U31, U63};
use fixed::{FixedI16, FixedI32, FixedI64, traits::Fixed};

use downcast_rs::DowncastSync;
use dyn_clone::DynClone;
use num_traits::NumCast;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

// //////////////////////////////////////////////////
// // IsDpsaFixed

pub fn float_to_fixed_floor<Fl, Fx>(x: Fl) -> Fx
where
    Fl: num_traits::Float,
    Fx: Fixed,
    Fx::Bits : num_traits::NumCast,
{
    float_to_fixed_with(x, Fl::floor)
}

pub fn float_to_fixed_ceil<Fl, Fx>(x: Fl) -> Fx
where
    Fl: num_traits::Float,
    Fx: Fixed,
    Fx::Bits : num_traits::NumCast,
{
    float_to_fixed_with(x, Fl::ceil)
}

fn float_to_fixed_with<Fl, Fx, Fun>(x: Fl, f: Fun) -> Fx
    where
        Fl: num_traits::Float,
        Fx: Fixed,
        Fx::Bits : num_traits::NumCast,
        Fun: FnOnce(Fl) -> Fl,
{
    // the number of bits of our fixed type representation
    let n = Fx::Signed::FRAC_NBITS + Fx::Signed::INT_NBITS;

    // a fixed number is ostensibly an integer i in the range [-2^(n-1)..2^(n-1)]
    // We do a manual conversion:
    // - the float `x` should be in the range [-1..1]
    // - we expand to the range [-2^(n-1)..2^(n-1)]
    let x = x * Fl::from(2u64.pow(n-1)).unwrap();

    // - we apply the postprocessing function
    //   this could be floor or ceil, in order to remove the fractional part
    let x = f(x);

    // - convert to integer
    let x = <Fx::Bits as NumCast>::from(x).unwrap();

    // - turn bitwise rep into fixed
    let bits : Fx = Fixed::from_bits(x);

    bits
}


#[cfg(test)]
mod tests
{
    use super::*;
    // use ::fixed::types::extra::{U15, U31, U63};
    // use ::fixed::{FixedI16, FixedI32, FixedI64};
    use fixed_macro::fixed;

    #[test]
    fn float_to_fixed_floor_test()
    {
        // left: 2^(-15)
        // right: 2^(-15)
        assert_eq!(float_to_fixed_floor::<f32,Fixed16>(0.000030517578125), fixed!(0.000030517578125: I1F15));

        // left: 2^(-16)
        // right: 0
        assert_eq!(float_to_fixed_floor::<f32,Fixed16>(0.0000152587890625), fixed!(0.0: I1F15));

        // left: 2^(-15) + 2^(-16) + 2^(-17) + 2^(-18)
        // right: 2^(-15)
        assert_eq!(float_to_fixed_floor::<f32,Fixed16>(0.000057220458984375), fixed!(0.000030517578125: I1F15));
    }

    #[test]
    fn float_to_fixed_ceil_test()
    {
        // left: 2^(-15)
        // right: 2^(-15)
        assert_eq!(float_to_fixed_ceil::<f32,Fixed16>(0.000030517578125), fixed!(0.000030517578125: I1F15));

        // left: 2^(-16)
        // right: 2^(-15)
        assert_eq!(float_to_fixed_ceil::<f32,Fixed16>(0.0000152587890625), fixed!(0.000030517578125: I1F15));

        // left: 2^(-15) + 2^(-20)
        // right: 2^(-14)
        assert_eq!(float_to_fixed_ceil::<f32,Fixed16>(0.00003147125244140625), fixed!(0.00006103515625: I1F15));
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









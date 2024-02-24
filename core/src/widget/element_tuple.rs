use smallvec::SmallVec;

use crate::{Context, Id};
use super::{flex::FlexChild, Element};

pub trait FlexElementTuple {
    fn make(
        self,
        ctx: &mut Context,
        parent: Id,
        children: &mut SmallVec<[FlexChild; 8]>
    );
}

macro_rules! flex_tuple {
    ( $($ty:ident),* ; $($n:tt),* ) => {
        impl< $( $ty: Element, )* > FlexElementTuple for ( $(($ty, f32),)* ) {
            fn make(
                self,
                ctx: &mut Context,
                parent: Id,
                children: &mut SmallVec<[FlexChild; 8]>
            ) {
                $(
                    children.push(FlexChild {
                        id: ctx.new_child(parent, self.$n.0).into(),
                        flex: self.$n.1
                    });
                )*
            }
        }   
    };
}

impl<T0: Element> FlexElementTuple for (T0, f32) {
    fn make(self, ctx: &mut Context, parent: Id,
        children: &mut SmallVec<[FlexChild; 8]>) {
        children.push(FlexChild {
            id: ctx.new_child(parent, self.0).into(),
            flex: self.1
        });
    }
}

flex_tuple!(T0, T1; 0, 1);
flex_tuple!(T0, T1, T2; 0, 1, 2);
flex_tuple!(T0, T1, T2, T3; 0, 1, 2, 3);
flex_tuple!(T0, T1, T2, T3, T4; 0, 1, 2, 3, 4);
flex_tuple!(T0, T1, T2, T3, T4, T5; 0, 1, 2, 3, 4, 5);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6; 0, 1, 2, 3, 4, 5, 6);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7; 0, 1, 2, 3, 4, 5, 6, 7);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8; 0, 1, 2, 3, 4, 5, 6, 7, 8);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14);
flex_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);

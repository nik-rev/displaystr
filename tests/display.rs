#![cfg(test)]
use displaystr::display;

#[test]
fn ui() {
    let harness = trybuild::TestCases::new();
    harness.compile_fail("tests/ui/*.rs");
}

#[test]
fn unit_variant() {
    #[rustfmt::skip]
    #[display(doc)]
    enum UnitVariant {
        A = "unit variant",
        B() = "unit variant with `()`",
        C{} = "unit variant with `{{}}`"
    }

    assert_eq!(UnitVariant::A.to_string(), "unit variant");
    assert_eq!(UnitVariant::B().to_string(), "unit variant with `()`");
    assert_eq!(UnitVariant::C {}.to_string(), "unit variant with `{}`");
}

#[test]
fn tuple_variant() {
    #[rustfmt::skip]
    #[display(doc)]
    enum TupleVariant {
        A(u32) = "tuple 1: {_0}",
        B(u32,) = "tuple 1 with trailing comma: {_0}",
        C(u32, String) = "tuple 2: {_0}, {_1}",
        D(u32, String,) = ("tuple 2 with trailing comma: {_0}, {}", _1),
    }

    assert_eq!(TupleVariant::A(1).to_string(), "tuple 1: 1");
    assert_eq!(
        TupleVariant::B(2).to_string(),
        "tuple 1 with trailing comma: 2"
    );
    assert_eq!(
        TupleVariant::C(3, "a".to_string()).to_string(),
        "tuple 2: 3, a"
    );
    assert_eq!(
        TupleVariant::D(4, "b".to_string()).to_string(),
        "tuple 2 with trailing comma: 4, b"
    );
}

#[test]
fn struct_variant() {
    #[rustfmt::skip]
    #[display(doc)]
    enum StructVariant {
        A { first: u32 } = "tuple 1: {first}",
        B { first: u32, } = "tuple 1 with trailing comma: {first}",
        C { first: u32, second: String } = "tuple 2: {first}, {second}",
        D { first: u32, second: String, } = ("tuple 2 with trailing comma: {first}, {}", second)
    }

    assert_eq!(StructVariant::A { first: 1 }.to_string(), "tuple 1: 1");
    assert_eq!(
        StructVariant::B { first: 2 }.to_string(),
        "tuple 1 with trailing comma: 2"
    );
    assert_eq!(
        StructVariant::C {
            first: 3,
            second: "a".to_string()
        }
        .to_string(),
        "tuple 2: 3, a"
    );
    assert_eq!(
        StructVariant::D {
            first: 4,
            second: "b".to_string()
        }
        .to_string(),
        "tuple 2 with trailing comma: 4, b"
    );
}

use std::ops::Not;

use crate::data_types::{ScalarType, Type, BIT};
use crate::data_values::Value;
use crate::errors::Result;
use crate::graphs::{Graph, Node};
use crate::typed_value::TypedValue;

/// This function tests that two given inputs containing arrays or scalars of bitstrings
/// are compatible for binary custom operations on bits that involve broadcasting,
/// e.g. comparison and binary addition.
pub(super) fn validate_arguments_in_broadcast_bit_ops(
    arguments_types: Vec<Type>,
    custom_op_name: &str,
) -> Result<()> {
    if arguments_types.len() != 2 {
        return Err(runtime_error!("Invalid number of arguments"));
    }
    let mut are_valid_inputs: bool = true;
    let mut error_message: String = format!("{}: ", custom_op_name);
    match (&arguments_types[0], &arguments_types[1]) {
        (Type::Array(shape0, scalar_type0), Type::Array(shape1, scalar_type1)) => {
            if shape0[shape0.len() - 1] != shape1[shape1.len() - 1] {
                are_valid_inputs = false;
                error_message.push_str("Input arrays' last dimensions are not the same");
            } else if *scalar_type0 != BIT {
                are_valid_inputs = false;
                error_message.push_str("Input array [0]'s ScalarType is not BIT");
            } else if *scalar_type1 != BIT {
                are_valid_inputs = false;
                error_message.push_str("Input array [1]'s ScalarType is not BIT");
            }
        }
        _ => {
            are_valid_inputs = false;
            error_message.push_str("Invalid input argument type, expected Array type");
        }
    }
    if !are_valid_inputs {
        Err(runtime_error!("{}", error_message))
    } else {
        Ok(())
    }
}

/// Panics if `x` is not an array.
pub fn pull_out_bits(x: Node) -> Result<Node> {
    let shape = x.get_type()?.get_dimensions();
    if shape.len() == 1 {
        Ok(x)
    } else {
        let mut axes_permutation = vec![shape.len() as u64 - 1];
        axes_permutation.extend(0..shape.len() as u64 - 1);
        Ok(x.permute_axes(axes_permutation)?)
    }
}

/// Panics if `x` is not an array.
pub fn put_in_bits(x: Node) -> Result<Node> {
    let shape = x.get_type()?.get_dimensions();
    if shape.len() == 1 {
        Ok(x)
    } else {
        let mut axes_permutation: Vec<u64> = (1..shape.len()).map(|x| x as u64).collect();
        axes_permutation.push(0);
        Ok(x.permute_axes(axes_permutation)?)
    }
}

pub fn zeros(g: &Graph, t: Type) -> Result<Node> {
    g.constant(t.clone(), Value::zero_of_type(t))
}

pub fn zeros_like(x: Node) -> Result<Node> {
    zeros(&x.get_graph(), x.get_type()?)
}

pub fn constant(g: &Graph, v: TypedValue) -> Result<Node> {
    g.constant(v.t, v.value)
}

pub fn constant_scalar<T: TryInto<u64> + Not<Output = T> + TryInto<u8> + Copy>(
    g: &Graph,
    value: T,
    st: ScalarType,
) -> Result<Node> {
    constant(g, TypedValue::from_scalar(value, st)?)
}

pub fn multiply_bit_and_number(bit: Node, number: Node) -> Result<Node> {
    // TODO: switch to MixedMultiply once it is supported in MPC.
    let g = bit.get_graph();
    let mut bits = vec![bit.clone()];
    let zero = zeros_like(bit.clone())?;
    for _ in 1..64 {
        bits.push(zero.clone());
    }
    let bit_arithmetic = put_in_bits(g.create_vector(bit.get_type()?, bits)?.vector_to_array()?)?
        .b2a(number.get_type()?.get_scalar_type())?;
    bit_arithmetic.multiply(number)
}

pub fn multiply_fixed_point(node1: Node, node2: Node, precision: u64) -> Result<Node> {
    node1.multiply(node2)?.truncate(1 << precision)
}

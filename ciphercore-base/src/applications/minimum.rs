//! Minimum of an integer array
use crate::custom_ops::CustomOperation;
use crate::data_types::{array_type, ScalarType, BIT};
use crate::errors::Result;
use crate::graphs::{Context, Graph, SliceElement};
use crate::ops::min_max::Min;

/// Creates a graph that finds the minimum of an array.
///
/// # Arguments
///
/// * `context` - context where a minimum graph should be created
/// * `n` - number of elements of an array (i.e., 2<sup>n</sup>)
/// * `st` - scalar type of array elements
///
/// # Returns
///
/// Graph that finds the minimum of an array
pub fn create_minimum_graph(context: Context, n: u64, st: ScalarType) -> Result<Graph> {
    // Get sign of the input scalar type that indicates whether signed comparisons should be computed
    let signed_comparison = st.get_signed();

    // Create a graph in a given context that will be used for finding the minimum
    let g = context.create_graph()?;

    // Create the type of the input array with `2^n` elements.
    let input_type = array_type(vec![1 << n], st.clone());

    // Add an input node to the empty graph g created above.
    // This input node requires the input array type generated previously.
    let input_array = g.input(input_type)?;

    // To find the minimum of an array, we resort to the custom operation Min (see ops.rs) that accepts only binary input.
    // Thus, we need to convert the input in the binary form (if necessary).
    let mut binary_array = if st != BIT {
        input_array.a2b()?
    } else {
        input_array
    };

    // We find the minimum using the tournament method. This allows to reduce the graph size to O(n) from O(2<sup>n</sup>) nodes.
    // Namely, we split the input array into pairs, find a minimum within each pair and create a new array from these minima.
    // Then, we repeat this procedure for the new array.
    // For example, let [2,7,0,3,11,5,0,4] be an input array.
    // The 1st iteration yields [min(2,11), min(7,5), min(0,0), min(3,4)] = [2,5,0,3]
    // The 2nd iteration results in [min(2,0), min(5,3)] = [0,3]
    // The 3rd iteration returns [min(0,3)] = [0]
    for level in (0..n).rev() {
        // Extract the first half of the array using the [Graph::get_slice] operation.
        // Our slicing conventions follow [the NumPy rules](https://numpy.org/doc/stable/user/basics.indexing.html).
        let half1 =
            binary_array.get_slice(vec![SliceElement::SubArray(None, Some(1 << level), None)])?;
        // Extract the first half of the array using the [Graph::get_slice] operation.
        let half2 =
            binary_array.get_slice(vec![SliceElement::SubArray(Some(1 << level), None, None)])?;
        // Compare the first half with the second half elementwise to find minimums.
        // This is done via the custom operation Min (see ops.rs).
        binary_array = g.custom_op(
            CustomOperation::new(Min { signed_comparison }),
            vec![half1, half2],
        )?;
    }
    // Convert output from the binary form to the arithmetic form
    let output = if st != BIT {
        binary_array.b2a(st)?
    } else {
        binary_array
    };
    // Before computation every graph should be finalized, which means that it should have a designated output node.
    // This can be done by calling `g.set_output_node(output)?` or as below.
    output.set_as_output()?;
    // Finalization checks that the output node of the graph g is set. After finalization the graph can't be changed.
    g.finalize()?;

    Ok(g)
}

#[cfg(test)]
mod tests {
    use crate::custom_ops::run_instantiation_pass;
    use crate::data_types::{INT32, UINT32};
    use crate::data_values::Value;
    use crate::evaluators::random_evaluate;
    use crate::graphs::create_context;
    use std::ops::Not;

    use super::*;

    fn test_minimum_helper<T: TryInto<u64> + Not<Output = T> + TryInto<u8> + Copy>(
        input_value: &[T],
        n: u64,
        st: ScalarType,
    ) -> Value {
        || -> Result<Value> {
            let c = create_context()?;
            let g = create_minimum_graph(c.clone(), n, st.clone())?;
            g.set_as_main()?;
            c.finalize()?;
            let mapped_c = run_instantiation_pass(c)?.get_context();
            let mapped_g = mapped_c.get_main_graph()?;

            let input_type = array_type(vec![n], st.clone());
            let val = Value::from_flattened_array(input_value, input_type.get_scalar_type())?;
            random_evaluate(mapped_g, vec![val])
        }()
        .unwrap()
    }

    #[test]
    fn test_minimum() {
        || -> Result<()> {
            assert!(
                test_minimum_helper(
                    &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
                    4,
                    UINT32
                ) == Value::from_flattened_array(&[1], UINT32)?
            );
            Ok(())
        }()
        .unwrap();
        || -> Result<()> {
            assert!(
                test_minimum_helper(
                    &[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16],
                    4,
                    INT32
                ) == Value::from_flattened_array(&[-15], INT32)?
            );
            Ok(())
        }()
        .unwrap();
        || -> Result<()> {
            assert!(
                test_minimum_helper(&[0, 1, 1, 0, 1, 1, 0, 0], 3, BIT)
                    == Value::from_flattened_array(&[0], BIT)?
            );
            Ok(())
        }()
        .unwrap();
    }
}

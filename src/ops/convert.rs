use crate::ops::{DataType, Input, InputList, IntoOpResult, OpError, Operator, Output};

#[derive(Debug)]
pub struct Cast {
    pub to: DataType,
}

impl Operator for Cast {
    fn name(&self) -> &str {
        "Cast"
    }

    fn run(&self, inputs: InputList) -> Result<Vec<Output>, OpError> {
        let input = inputs.require(0)?;
        let result: Output = match input {
            Input::IntTensor(t) => match self.to {
                DataType::Int32 => (*t).clone().into(),
                DataType::Float => t.map(|x| x as f32).into(),
            },
            Input::FloatTensor(t) => match self.to {
                DataType::Int32 => t.map(|x| x as i32).into(),
                DataType::Float => (*t).clone().into(),
            },
        };
        result.into_op_result()
    }

    fn can_run_in_place(&self) -> bool {
        true
    }

    fn run_in_place(&self, input: Output, _: InputList) -> Result<Output, OpError> {
        match (input, self.to) {
            (Output::IntTensor(t), DataType::Int32) => Ok(t.into()),
            (Output::FloatTensor(t), DataType::Float) => Ok(t.into()),
            (input, _) => self
                .run(InputList::from(&[(&input).into()]))
                .map(|mut outputs| outputs.remove(0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ops::{Cast, DataType, Input, InputList, Operator};
    use crate::tensor::from_vec;
    use crate::test_util::expect_equal;

    #[test]
    fn test_cast() -> Result<(), String> {
        let int_input = from_vec(vec![1, 2, 3]);
        let float_input = from_vec(vec![1.0, 2.0, 3.0]);

        // No-op cast from int32 => int32
        let cast_to_int = Cast {
            to: DataType::Int32,
        };
        let result = cast_to_int
            .run(InputList::from(&[Input::IntTensor(&int_input)]))
            .unwrap()
            .remove(0)
            .into_int()
            .unwrap();

        // Flooring cast from float => int32
        assert_eq!(result, int_input);
        let result = cast_to_int
            .run(InputList::from(&[Input::FloatTensor(&float_input)]))
            .unwrap()
            .remove(0)
            .into_int()
            .unwrap();
        assert_eq!(&result, &int_input);

        // No-op cast from float => float
        let cast_to_float = Cast {
            to: DataType::Float,
        };
        let result = cast_to_float
            .run(InputList::from(&[Input::FloatTensor(&float_input)]))
            .unwrap()
            .remove(0)
            .into_float()
            .unwrap();
        expect_equal(&result, &float_input)?;

        // Cast from int32 => float
        let result = cast_to_float
            .run(InputList::from(&[Input::IntTensor(&int_input)]))
            .unwrap()
            .remove(0)
            .into_float()
            .unwrap();
        expect_equal(&result, &float_input)
    }

    #[test]
    fn test_cast_out_of_range() -> Result<(), String> {
        let int_input = from_vec(vec![i32::MIN, i32::MAX]);

        // Out-of-range cast from int => float. This will simply lose some
        // significant digits.
        let cast_to_float = Cast {
            to: DataType::Float,
        };
        let result = cast_to_float
            .run(InputList::from(&[(&int_input).into()]))
            .unwrap()
            .remove(0)
            .into_float()
            .unwrap();
        expect_equal(&result, &from_vec(vec![-2147483600.0, 2147483600.0]))?;

        // Out-of-range cast from float => int.
        let float_input = from_vec(vec![f32::MIN, f32::MAX]);
        let cast_to_int = Cast {
            to: DataType::Int32,
        };
        let result = cast_to_int
            .run(InputList::from(&[(&float_input).into()]))
            .unwrap()
            .remove(0)
            .into_int()
            .unwrap();
        assert_eq!(&result, &from_vec(vec![i32::MIN, i32::MAX]));

        Ok(())
    }
}

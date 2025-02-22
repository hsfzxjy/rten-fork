use rten_tensor::TensorView;

use crate::graph::{CaptureEnv, Graph, RunError, RunOptions};
use crate::ops::{InputList, OpError, Operator, Output, OutputList};
use crate::tensor_pool::TensorPool;

fn output_list_from_vec(xs: Vec<Output>) -> OutputList {
    xs.into_iter().collect()
}

fn run_error_from_op_error(error: OpError) -> RunError {
    RunError::OperatorError {
        name: "If".to_string(),
        error,
    }
}

pub struct If {
    pub then_branch: Graph,
    pub else_branch: Graph,
}

impl std::fmt::Debug for If {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "If {{ ... }}")
    }
}

impl Operator for If {
    fn name(&self) -> &str {
        "If"
    }

    fn run(&self, _pool: &TensorPool, _inputs: InputList) -> Result<OutputList, OpError> {
        Err(OpError::InvalidValue(
            "operator must be run with `run_subgraph`",
        ))
    }

    fn has_subgraph(&self) -> bool {
        true
    }

    fn run_subgraph(
        &self,
        pool: &TensorPool,
        inputs: InputList,
        captures: &CaptureEnv,
        run_opts: Option<RunOptions>,
    ) -> Result<OutputList, RunError> {
        let cond: TensorView<i32> = inputs.require_as(0).map_err(run_error_from_op_error)?;
        let Some(cond_bool) = cond.item().copied() else {
            return Err(run_error_from_op_error(OpError::InvalidValue(
                "cond must be a single value",
            )));
        };

        if cond_bool != 0 {
            self.then_branch
                .run_subgraph(
                    Vec::new(),
                    self.then_branch.output_ids(),
                    captures,
                    Some(pool),
                    run_opts,
                )
                .map(output_list_from_vec)
        } else {
            self.else_branch
                .run_subgraph(
                    Vec::new(),
                    self.else_branch.output_ids(),
                    captures,
                    Some(pool),
                    run_opts,
                )
                .map(output_list_from_vec)
        }
    }
}

use pyo3::exceptions::PyNotImplementedError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use crate::build_tools::py_schema_err;
use crate::errors::{ErrorType, ValError, ValResult};
use crate::input::Input;
use crate::tools::SchemaDict;

use super::{BuildValidator, CombinedValidator, DefinitionsBuilder, ValidationState, Validator};

#[derive(Debug, Clone)]
pub struct IsInstanceValidator {
    class: PyObject,
    class_repr: String,
    name: String,
}

impl BuildValidator for IsInstanceValidator {
    const EXPECTED_TYPE: &'static str = "is-instance";

    fn build(
        schema: &Bound<'_, PyDict>,
        _config: Option<&Bound<'_, PyDict>>,
        _definitions: &mut DefinitionsBuilder<CombinedValidator>,
    ) -> PyResult<CombinedValidator> {
        let py = schema.py();
        let cls_key = intern!(py, "cls");
        let class = schema.get_as_req(cls_key)?;

        // test that class works with isinstance to avoid errors at call time, reuse cls_key since it doesn't
        // matter what object is being checked
        if cls_key.is_instance(&class).is_err() {
            return py_schema_err!("'cls' must be valid as the first argument to 'isinstance'");
        }

        let class_repr = match schema.get_as(intern!(py, "cls_repr"))? {
            Some(s) => s,
            None => match class.extract::<&PyType>() {
                Ok(t) => t.qualname()?.to_string(),
                Err(_) => class.repr()?.extract()?,
            },
        };
        let name = format!("{}[{class_repr}]", Self::EXPECTED_TYPE);
        Ok(Self {
            class: class.into(),
            class_repr,
            name,
        }
        .into())
    }
}

impl_py_gc_traverse!(IsInstanceValidator { class });

impl Validator for IsInstanceValidator {
    fn validate<'data>(
        &self,
        py: Python<'data>,
        input: &'data impl Input<'data>,
        _state: &mut ValidationState,
    ) -> ValResult<PyObject> {
        if !input.is_python() {
            return Err(ValError::InternalErr(PyNotImplementedError::new_err(
                "Cannot check isinstance when validating from json, \
                            use a JsonOrPython validator instead.",
            )));
        }

        let ob: Py<PyAny> = input.to_object(py);
        match ob.bind(py).is_instance(self.class.bind(py))? {
            true => Ok(ob),
            false => Err(ValError::new(
                ErrorType::IsInstanceOf {
                    class: self.class_repr.clone(),
                    context: None,
                },
                input,
            )),
        }
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

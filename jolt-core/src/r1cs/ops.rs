//! Defines the Linear Combination (LC) object and associated operations.
//! A LinearCombination is a vector of Terms, where each Term is a pair of a Variable and a coefficient.

use crate::field::JoltField;
use std::fmt::Debug;
use strum::{EnumCount, IntoEnumIterator};

pub trait ConstraintInput:
    Clone
    + Copy
    + Debug
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + IntoEnumIterator
    + EnumCount
    + Into<usize>
    + Sync
    + 'static
{
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variable<I: ConstraintInput> {
    Input(I),
    Auxiliary(usize),
    Constant,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Term<I: ConstraintInput>(pub Variable<I>, pub i64);

/// Linear Combination of terms.
#[derive(Clone)]
pub struct LC<I: ConstraintInput>(Vec<Term<I>>);

impl<I: ConstraintInput> LC<I> {
    pub fn new(terms: Vec<Term<I>>) -> Self {
        #[cfg(test)]
        Self::assert_no_duplicate_terms(&terms);

        let mut sorted_terms = terms;
        sorted_terms.sort_by(|a, b| a.0.cmp(&b.0));
        LC(sorted_terms)
    }

    pub fn zero() -> Self {
        LC::new(vec![])
    }

    pub fn terms(&self) -> &[Term<I>] {
        &self.0
    }

    pub fn constant_term(&self) -> Option<&Term<I>> {
        self.0
            .last()
            .filter(|term| matches!(term.0, Variable::Constant))
    }

    pub fn to_field_elements<F: JoltField>(&self) -> Vec<F> {
        self.terms()
            .iter()
            .map(|term| from_i64::<F>(term.1))
            .collect()
    }

    pub fn num_terms(&self) -> usize {
        self.0.len()
    }

    pub fn num_vars(&self) -> usize {
        self.0
            .iter()
            .filter(|term| matches!(term.0, Variable::Auxiliary(_) | Variable::Input(_)))
            .count()
    }

    pub fn evaluate<F: JoltField>(&self, values: &[F]) -> F {
        let num_vars = self.num_vars();
        assert_eq!(num_vars, values.len());

        let mut var_index = 0;
        let mut result = F::zero();
        for term in self.terms().iter() {
            match term.0 {
                Variable::Input(_) => {
                    result += values[var_index] * from_i64::<F>(term.1);
                    var_index += 1;
                }
                Variable::Auxiliary(_) => {
                    result += values[var_index] * from_i64::<F>(term.1);
                    var_index += 1;
                }
                Variable::Constant => result += from_i64::<F>(term.1),
            }
        }
        result
    }

    #[cfg(test)]
    fn assert_no_duplicate_terms(terms: &[Term<I>]) {
        let mut term_vec = Vec::new();
        for term in terms {
            if term_vec.contains(&term.0) {
                panic!("Duplicate variable found in terms: {:?}", term.0);
            } else {
                term_vec.push(term.0);
            }
        }
    }
}

impl<I: ConstraintInput> std::fmt::Debug for LC<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LC(")?;
        for (index, term) in self.0.iter().enumerate() {
            if index > 0 {
                write!(f, " + ")?;
            }
            write!(f, "{:?}", term)?;
        }
        write!(f, ")")
    }
}

impl<I: ConstraintInput> std::fmt::Debug for Term<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}*{:?}", self.1, self.0)
    }
}

// TODO(sragss): Move this onto JoltField
pub fn from_i64<F: JoltField>(val: i64) -> F {
    if val > 0 {
        F::from_u64(val as u64).unwrap()
    } else {
        // TODO(sragss): THIS DOESN'T WORK FOR BINIUS
        F::zero() - F::from_u64(-(val) as u64).unwrap()
    }
}

// Arithmetic for LC

impl<I, T> std::ops::Add<T> for LC<I>
where
    I: ConstraintInput,
    T: Into<LC<I>>,
{
    type Output = Self;

    fn add(self, other: T) -> Self::Output {
        let other_lc: LC<I> = other.into();
        let mut combined_terms = self.0;
        // TODO(sragss): Can be made more efficient by assuming sorted
        for other_term in other_lc.terms() {
            if let Some(term) = combined_terms
                .iter_mut()
                .find(|term| term.0 == other_term.0)
            {
                term.1 += other_term.1;
            } else {
                combined_terms.push(*other_term);
            }
        }
        LC::new(combined_terms)
    }
}

impl<I, T> std::ops::Add<T> for Term<I>
where
    I: ConstraintInput,
    T: Into<LC<I>>,
{
    type Output = LC<I>;

    fn add(self, other: T) -> Self::Output {
        let other_lc: LC<I> = other.into();
        LC::new(vec![self]) + other_lc
    }
}

impl<I, T> std::ops::Add<T> for Variable<I>
where
    I: ConstraintInput,
    T: Into<LC<I>>,
{
    type Output = LC<I>;

    fn add(self, other: T) -> Self::Output {
        let other_lc: LC<I> = other.into();
        LC::new(vec![Term(self, 1)]) + other_lc
    }
}

impl<I: ConstraintInput> std::ops::Neg for LC<I> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let negated_terms: Vec<Term<I>> = self.0.into_iter().map(|term| -term).collect();
        LC::new(negated_terms)
    }
}

impl<I: ConstraintInput, T: Into<LC<I>>> std::ops::Sub<T> for LC<I> {
    type Output = Self;

    fn sub(self, other: T) -> Self::Output {
        let other: LC<I> = other.into();
        let negated_other = -other;
        self + negated_other
    }
}

// Arithmetic for Term<I>

impl<I: ConstraintInput> std::ops::Neg for Term<I> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Term(self.0, -self.1)
    }
}

impl<I: ConstraintInput> From<i64> for Term<I> {
    fn from(val: i64) -> Self {
        Term(Variable::Constant, val)
    }
}

impl<I: ConstraintInput> From<Variable<I>> for Term<I> {
    fn from(val: Variable<I>) -> Self {
        Term(val, 1)
    }
}

impl<I: ConstraintInput> From<(Variable<I>, i64)> for Term<I> {
    fn from(val: (Variable<I>, i64)) -> Self {
        Term(val.0, val.1)
    }
}

impl<I: ConstraintInput> std::ops::Sub for Variable<I> {
    type Output = LC<I>;

    fn sub(self, other: Self) -> Self::Output {
        LC::new(vec![Term(self, 1), Term(other, -1)])
    }
}

// Into<LC<I>>

impl<I: ConstraintInput> From<i64> for LC<I> {
    fn from(val: i64) -> Self {
        LC::new(vec![Term(Variable::Constant, val)])
    }
}

impl<I: ConstraintInput> From<Variable<I>> for LC<I> {
    fn from(val: Variable<I>) -> Self {
        LC::new(vec![Term(val, 1)])
    }
}

impl<I: ConstraintInput> From<Term<I>> for LC<I> {
    fn from(val: Term<I>) -> Self {
        LC::new(vec![val])
    }
}

impl<I: ConstraintInput> From<Vec<Term<I>>> for LC<I> {
    fn from(val: Vec<Term<I>>) -> Self {
        LC::new(val)
    }
}

// Generic arithmetic for Variable<I>

impl<I: ConstraintInput> std::ops::Mul<i64> for Variable<I> {
    type Output = Term<I>;

    fn mul(self, other: i64) -> Self::Output {
        Term(self, other)
    }
}

impl<I: ConstraintInput> std::ops::Mul<Variable<I>> for i64 {
    type Output = Term<I>;

    fn mul(self, other: Variable<I>) -> Self::Output {
        Term(other, self)
    }
}

/// Conversions and arithmetic for concrete ConstraintInput
#[macro_export]
macro_rules! impl_r1cs_input_lc_conversions {
    ($ConcreteInput:ty) => {
        impl Into<usize> for $ConcreteInput {
            fn into(self) -> usize {
                self as usize
            }
        }
        impl Into<$crate::r1cs::ops::Variable<$ConcreteInput>> for $ConcreteInput {
            fn into(self) -> $crate::r1cs::ops::Variable<$ConcreteInput> {
                $crate::r1cs::ops::Variable::Input(self)
            }
        }

        impl Into<($crate::r1cs::ops::Variable<$ConcreteInput>, i64)> for $ConcreteInput {
            fn into(self) -> ($crate::r1cs::ops::Variable<$ConcreteInput>, i64) {
                ($crate::r1cs::ops::Variable::Input(self), 1)
            }
        }
        impl Into<$crate::r1cs::ops::Term<$ConcreteInput>> for $ConcreteInput {
            fn into(self) -> $crate::r1cs::ops::Term<$ConcreteInput> {
                $crate::r1cs::ops::Term($crate::r1cs::ops::Variable::Input(self), 1)
            }
        }

        impl Into<$crate::r1cs::ops::Term<$ConcreteInput>> for ($ConcreteInput, i64) {
            fn into(self) -> $crate::r1cs::ops::Term<$ConcreteInput> {
                $crate::r1cs::ops::Term($crate::r1cs::ops::Variable::Input(self.0), self.1)
            }
        }

        impl Into<$crate::r1cs::ops::LC<$ConcreteInput>> for $ConcreteInput {
            fn into(self) -> $crate::r1cs::ops::LC<$ConcreteInput> {
                $crate::r1cs::ops::Term($crate::r1cs::ops::Variable::Input(self), 1).into()
            }
        }

        impl Into<$crate::r1cs::ops::LC<$ConcreteInput>> for Vec<$ConcreteInput> {
            fn into(self) -> $crate::r1cs::ops::LC<$ConcreteInput> {
                let terms: Vec<$crate::r1cs::ops::Term<$ConcreteInput>> =
                    self.into_iter().map(Into::into).collect();
                $crate::r1cs::ops::LC::new(terms)
            }
        }

        impl<T: Into<$crate::r1cs::ops::LC<$ConcreteInput>>> std::ops::Add<T> for $ConcreteInput {
            type Output = $crate::r1cs::ops::LC<$ConcreteInput>;

            fn add(self, rhs: T) -> Self::Output {
                let lhs_lc: $crate::r1cs::ops::LC<$ConcreteInput> = self.into();
                let rhs_lc: $crate::r1cs::ops::LC<$ConcreteInput> = rhs.into();
                lhs_lc + rhs_lc
            }
        }

        impl<T: Into<$crate::r1cs::ops::LC<$ConcreteInput>>> std::ops::Sub<T> for $ConcreteInput {
            type Output = $crate::r1cs::ops::LC<$ConcreteInput>;

            fn sub(self, rhs: T) -> Self::Output {
                let lhs_lc: $crate::r1cs::ops::LC<$ConcreteInput> = self.into();
                let rhs_lc: $crate::r1cs::ops::LC<$ConcreteInput> = rhs.into();
                lhs_lc + -rhs_lc
            }
        }

        impl std::ops::Mul<i64> for $ConcreteInput {
            type Output = $crate::r1cs::ops::Term<$ConcreteInput>;

            fn mul(self, rhs: i64) -> Self::Output {
                $crate::r1cs::ops::Term($crate::r1cs::ops::Variable::Input(self), rhs)
            }
        }

        impl std::ops::Mul<$ConcreteInput> for i64 {
            type Output = $crate::r1cs::ops::Term<$ConcreteInput>;

            fn mul(self, rhs: $ConcreteInput) -> Self::Output {
                $crate::r1cs::ops::Term($crate::r1cs::ops::Variable::Input(rhs), self)
            }
        }
        impl std::ops::Add<$ConcreteInput> for i64 {
            type Output = $crate::r1cs::ops::LC<$ConcreteInput>;

            fn add(self, rhs: $ConcreteInput) -> Self::Output {
                let term1 = $crate::r1cs::ops::Term($crate::r1cs::ops::Variable::Input(rhs), 1);
                let term2 = $crate::r1cs::ops::Term($crate::r1cs::ops::Variable::Constant, self);
                $crate::r1cs::ops::LC::new(vec![term1, term2])
            }
        }
    };
}

/// ```rust
/// use jolt_core::input_range;
/// use jolt_core::r1cs::ops::{ConstraintInput, Variable};
/// # use strum_macros::{EnumCount, EnumIter};
///
/// # #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, EnumCount, EnumIter)]
/// #[repr(usize)]
/// pub enum Inputs {
///     A,
///     B,
///     C,
///     D
/// }
/// #
/// # impl Into<usize> for Inputs {
/// #   fn into(self) -> usize {
/// #       self as usize
/// #   }
/// # }
/// #
/// impl ConstraintInput for Inputs {};
///
/// let range = input_range!(Inputs::B, Inputs::D);
/// let expected_range = [Variable::Input(Inputs::B), Variable::Input(Inputs::C), Variable::Input(Inputs::D)];
/// assert_eq!(range, expected_range);
/// ```
#[macro_export]
macro_rules! input_range {
    ($start:path, $end:path) => {{
        let mut arr = [Variable::Input($start); ($end as usize) - ($start as usize) + 1];
        #[allow(clippy::missing_transmute_annotations)]
        for i in ($start as usize)..=($end as usize) {
            arr[i - ($start as usize)] =
                Variable::Input(unsafe { std::mem::transmute::<usize, _>(i) });
        }
        arr
    }};
}

/// Used to fix an aux variable to a constant index at runtime for use elsewhere (largely OffsetEqConstraints).
#[macro_export]
macro_rules! assert_static_aux_index {
    ($var:expr, $index:expr) => {{
        if let Variable::Auxiliary(aux_index) = $var {
            assert_eq!(aux_index, $index, "Unexpected auxiliary index");
        } else {
            panic!("Variable is not of variant type Variable::Auxiliary");
        }
    }};
}

#[cfg(test)]
mod test {
    use strum_macros::{EnumCount, EnumIter};

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, EnumCount, EnumIter)]
    #[repr(usize)]
    enum Inputs {
        A,
        B,
        C,
        D,
    }

    impl From<Inputs> for usize {
        fn from(val: Inputs) -> Self {
            val as usize
        }
    }
    impl ConstraintInput for Inputs {}

    #[test]
    fn variable_ordering() {
        let mut variables: Vec<Variable<Inputs>> = vec![
            Variable::Auxiliary(10),
            Variable::Auxiliary(5),
            Variable::Constant,
            Variable::Input(Inputs::C),
            Variable::Input(Inputs::B),
        ];
        let expected_sort: Vec<Variable<Inputs>> = vec![
            Variable::Input(Inputs::B),
            Variable::Input(Inputs::C),
            Variable::Auxiliary(5),
            Variable::Auxiliary(10),
            Variable::Constant,
        ];
        variables.sort();
        assert_eq!(variables, expected_sort);
    }

    #[test]
    fn lc_sorting() {
        let variables: Vec<Variable<Inputs>> = vec![
            Variable::Auxiliary(10),
            Variable::Auxiliary(5),
            Variable::Constant,
            Variable::Input(Inputs::C),
            Variable::Input(Inputs::B),
        ];

        let expected_sort: Vec<Variable<Inputs>> = vec![
            Variable::Input(Inputs::B),
            Variable::Input(Inputs::C),
            Variable::Auxiliary(5),
            Variable::Auxiliary(10),
            Variable::Constant,
        ];
        let expected_sorted_terms: Vec<Term<Inputs>> = expected_sort
            .into_iter()
            .map(|variable| variable.into())
            .collect();

        let terms = variables
            .into_iter()
            .map(|variable| variable.into())
            .collect();
        let lc = LC::new(terms);
        assert_eq!(lc.terms(), expected_sorted_terms);
    }
}

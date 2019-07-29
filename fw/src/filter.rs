use arraydeque::ArrayDeque;
use generic_array::GenericArray;
use typenum::Unsigned;
use core::fmt;
use libm::F64Ext;

pub struct UniformAverageFilter<L: generic_array::ArrayLength<f64>> {
    points: ArrayDeque<GenericArray<f64, L>, arraydeque::Wrapping>,
}

impl <L: generic_array::ArrayLength<f64>> fmt::Display for UniformAverageFilter<L> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UniformAverageFilter<{}>([", <L as Unsigned>::to_u32())?;
        for (ix, p) in self.points.iter().enumerate() {
            if ix != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", p)?;
        }
        write!(f, "])")?;
        Ok(())
    }
}

impl<L: generic_array::ArrayLength<f64>> UniformAverageFilter<L> {
    pub fn new() -> Self {
        Self {
            points: ArrayDeque::new(),
        }
    }

    pub fn add(&mut self, process_variable: f64) -> Option<f64> {
        self.points.push_back(process_variable)
    }

    pub fn get(&self) -> f64 {
        self.points.iter().fold(0.0, |acc, point| {
            acc + *point
        }) / (self.points.len() as f64)
    }

    pub fn apply_adjustment(&mut self, adjustment: f64) {
        for p in self.points.iter_mut() {
            *p += adjustment;
        }
    }

    pub fn is_full(&self) -> bool {
        self.points.len() == self.points.capacity()
    }
}

pub struct ConvolutionFilter<L: generic_array::ArrayLength<f64>> {
    points: ArrayDeque<GenericArray<f64, L>, arraydeque::Wrapping>,
    filter_response: GenericArray<f64, L>,
}

impl<L: generic_array::ArrayLength<f64>> ConvolutionFilter<L> {
    pub fn new(filter_response: GenericArray<f64, L>) -> Self {
        Self {
            points: ArrayDeque::new(),
            filter_response,
        }
    }

    pub fn new_linear_ramp_up() -> Self {
        let l = <L as Unsigned>::to_u32();
        let length = f64::from(l);
        let filter_response = GenericArray::from_exact_iter((0..l).map(|ix| {
            f64::from(ix + 1) / length
        }));

        Self::new(filter_response.unwrap())
    }

    pub fn new_linear_ramp_down() -> Self {
        let l = <L as Unsigned>::to_u32();
        let length = f64::from(l);
        let filter_response = GenericArray::from_exact_iter((0..l).map(|ix| {
            f64::from(l - ix) / length
        }));

        Self::new(filter_response.unwrap())
    }

    pub fn add(&mut self, process_variable: f64) -> Option<f64> {
        self.points.push_front(process_variable)
    }

    pub fn get(&self) -> f64 {
        self.points.iter().zip(self.filter_response.iter()).fold(0.0, |acc, (point, filter)| {
            acc + (*point * *filter)
        })
    }

    pub fn apply_adjustment(&mut self, adjustment: f64) {
        for p in self.points.iter_mut() {
            *p += adjustment;
        }
    }

    pub fn is_full(&self) -> bool {
        self.points.len() == self.points.capacity()
    }
}

impl <L: generic_array::ArrayLength<f64>> fmt::Display for ConvolutionFilter<L> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ConvolutionFilter<{}>(f: [", <L as Unsigned>::to_u32())?;
        for (ix, p) in self.filter_response.iter().enumerate() {
            if ix != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", p)?;
        }
        write!(f, "], p: [")?;
        for (ix, p) in self.points.iter().enumerate() {
            if ix != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", p)?;
        }
        write!(f, "])")?;
        Ok(())
    }
}

pub struct ExponentialAverageFilter {
    process_variable: f64,
    alpha: f64,
    beta: f64,
}

impl ExponentialAverageFilter {
    pub fn new(tau: u32, initial_process_variable: f64) -> Self {
        let alpha = 1.0 - (-1.0 / tau as f64).exp();
        Self {
            alpha,
            beta: 1.0 - alpha,
            process_variable: initial_process_variable,
        }
    }

    pub fn add(&mut self, process_variable: f64) {
        self.process_variable = (self.process_variable * self.beta) + (process_variable * self.alpha);
    }

    pub fn get(&self) -> f64 {
        self.process_variable
    }

    pub fn apply_adjustment(&mut self, adjustment: f64) {
        self.process_variable += adjustment;
    }
}

#[cfg(test)]
mod tests {
    use crate::filter::{ConvolutionFilter, ExponentialAverageFilter};
    use typenum::consts::U4;
    use assert_approx_eq::assert_approx_eq;

    #[test]
    fn ramp_down_filter() {
        let mut filter = ConvolutionFilter::<U4>::new_linear_ramp_down();

        filter.add(1.0);
        assert_eq!(1.0, filter.get());

        filter.add(0.0);
        assert_eq!(0.75, filter.get());

        filter.add(0.0);
        assert_eq!(0.5, filter.get());

        filter.add(0.0);
        assert_eq!(0.25, filter.get());

        filter.add(0.0);
        assert_eq!(0.0, filter.get());
    }

    #[test]
    fn ramp_up_filter() {
        let mut filter = ConvolutionFilter::<U4>::new_linear_ramp_up();

        filter.add(1.0);
        assert_eq!(0.25, filter.get());

        filter.add(0.0);
        assert_eq!(0.5, filter.get());

        filter.add(0.0);
        assert_eq!(0.75, filter.get());

        filter.add(0.0);
        assert_eq!(1.0, filter.get());

        filter.add(0.0);
        assert_eq!(0.0, filter.get());
    }

    #[test]
    fn exponential_filter() {
        let mut filter = ExponentialAverageFilter::new(1, 1.0);

        filter.add(2.0);
        assert_approx_eq!(1.63, filter.get(), 0.01);

        filter.add(2.0);
        assert_approx_eq!(1.86, filter.get(), 0.01);

        filter.add(2.0);
        assert_approx_eq!(1.95, filter.get(), 0.01);
    }

    #[test]
    fn long_exponential_filter() {
        let mut filter = ExponentialAverageFilter::new(4, 1.0);

        filter.add(2.0);
        filter.add(2.0);
        filter.add(2.0);
        filter.add(2.0);
        assert_approx_eq!(1.63, filter.get(), 0.01);

        filter.add(2.0);
        filter.add(2.0);
        filter.add(2.0);
        filter.add(2.0);
        assert_approx_eq!(1.86, filter.get(), 0.01);
    }
}
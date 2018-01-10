use std::marker::PhantomData;
use std::io;
use super::{cvt, get_optional, Alignment, Geometry};

use libparted_sys::{ped_constraint_destroy, ped_constraint_done, ped_constraint_duplicate,
                    ped_constraint_init, ped_constraint_intersect, ped_constraint_is_solution,
                    ped_constraint_new, ped_constraint_new_from_max, ped_constraint_new_from_min,
                    ped_constraint_new_from_min_max, ped_constraint_solve_max,
                    ped_constraint_solve_nearest, PedConstraint};

pub(crate) enum ConstraintSource {
    Init,
    New,
}

pub struct Constraint<'a> {
    pub(crate) constraint: *mut PedConstraint,
    pub(crate) source: ConstraintSource,
    pub(crate) phantom: PhantomData<&'a PedConstraint>,
}

impl<'a> Constraint<'a> {
    fn new_(constraint: *mut PedConstraint, source: ConstraintSource) -> Constraint<'a> {
        Constraint {
            constraint,
            source,
            phantom: PhantomData,
        }
    }

    /// A convenience wrapper for `Constraint::init()`.
    ///
    /// Allocates a new piece of memory and initializes the constraint.
    pub fn new(
        start_align: &Alignment,
        end_align: &Alignment,
        start_range: &Geometry,
        end_range: &Geometry,
        min_size: i64,
        max_size: i64,
    ) -> io::Result<Constraint<'a>> {
        cvt(unsafe {
            ped_constraint_new(
                start_align.alignment,
                end_align.alignment,
                start_range.geometry,
                end_range.geometry,
                min_size,
                max_size,
            )
        }).map(|constraint| Constraint::new_(constraint, ConstraintSource::New))
    }

    /// Return a constraint that requires a region to be entirely contained inside `max`.
    pub fn new_from_max(max: &Geometry) -> io::Result<Constraint<'a>> {
        cvt(unsafe { ped_constraint_new_from_max(max.geometry) })
            .map(|constraint| Constraint::new_(constraint, ConstraintSource::New))
    }

    /// Return a constraint that requires a region to be entirely contained inside `min`.
    pub fn new_from_min(min: &Geometry) -> io::Result<Constraint<'a>> {
        cvt(unsafe { ped_constraint_new_from_min(min.geometry) })
            .map(|constraint| Constraint::new_(constraint, ConstraintSource::New))
    }

    /// Return a constraint that requires a region to be entirely contained inside `min` and `max'.
    pub fn new_from_min_max(min: &Geometry, max: &Geometry) -> io::Result<Constraint<'a>> {
        cvt(unsafe { ped_constraint_new_from_min_max(min.geometry, max.geometry) })
            .map(|constraint| Constraint::new_(constraint, ConstraintSource::New))
    }

    /// Initializes a pre-allocated piece of memory to contain a constraint with the supplied
    /// default values.
    pub fn init(
        &mut self,
        start_align: &Alignment,
        end_align: &Alignment,
        start_range: &Geometry,
        end_range: &Geometry,
        min_size: i64,
        max_size: i64,
    ) -> io::Result<()> {
        cvt(unsafe {
            ped_constraint_init(
                self.constraint,
                start_align.alignment,
                end_align.alignment,
                start_range.geometry,
                end_range.geometry,
                min_size,
                max_size,
            )
        })?;

        self.source = ConstraintSource::Init;
        Ok(())
    }

    pub fn start_align<'b>(&'b self) -> Alignment<'b> {
        Alignment::from_raw(unsafe { (*self.constraint).start_align })
    }

    pub fn end_align<'b>(&'b self) -> Alignment<'b> {
        Alignment::from_raw(unsafe { (*self.constraint).end_align })
    }

    pub fn start_range<'b>(&'b self) -> Geometry<'b> {
        Geometry::from_raw(unsafe { (*self.constraint).start_range })
    }

    pub fn end_range<'b>(&'b self) -> Geometry<'b> {
        Geometry::from_raw(unsafe { (*self.constraint).end_range })
    }

    pub fn min_size(&'a self) -> i64 {
        unsafe { (*self.constraint).min_size }
    }

    pub fn max_size(&'a self) -> i64 {
        unsafe { (*self.constraint).max_size }
    }

    /// Duplicates a constraint, if possible.
    pub fn duplicate<'b>(&self) -> io::Result<Constraint<'b>> {
        cvt(unsafe { ped_constraint_duplicate(self.constraint) })
            .map(|constraint| Constraint::new_(constraint, ConstraintSource::New))
    }

    /// If the supplied constraint intersects with our constraint, a constraint will
    /// be returned with the computed solution.
    pub fn intersect(&self, other: &Constraint) -> Option<Constraint<'a>> {
        get_optional(unsafe { ped_constraint_intersect(self.constraint, other.constraint) })
            .map(|constraint| Constraint::new_(constraint, ConstraintSource::New))
    }

    /// Check whether `geometry` satisfies the constraint.
    pub fn is_solution(&self, geometry: &Geometry) -> bool {
        unsafe { ped_constraint_is_solution(self.constraint, geometry.geometry) == 1 }
    }

    /// Find the largest region that satisfies a constraint.Alignment
    ///
    /// There might be more than one solution. This function makes no guarantees about which
    /// solutions it will choose in this case.
    pub fn solve_max(&self) -> Option<Geometry<'a>> {
        get_optional(unsafe { ped_constraint_solve_max(self.constraint) }).map(|geometry| {
            Geometry {
                geometry,
                phantom: PhantomData,
            }
        })
    }

    /// Return the nearest region to `geom` that satisfies the constraint.
    ///
    /// # Note:
    ///
    /// _Nearest_ is somewhat ambiguous. This function makes no guarantees
    /// about how this ambiguity is resolved.
    pub fn solve_nearest(&self, geom: &Geometry) -> Option<Geometry<'a>> {
        get_optional(unsafe { ped_constraint_solve_nearest(self.constraint, geom.geometry) }).map(
            |geometry| Geometry {
                geometry,
                phantom: PhantomData,
            },
        )
    }
}

impl<'a> Drop for Constraint<'a> {
    fn drop(&mut self) {
        unsafe {
            match self.source {
                ConstraintSource::Init => ped_constraint_done(self.constraint),
                ConstraintSource::New => ped_constraint_destroy(self.constraint),
            }
        }
    }
}

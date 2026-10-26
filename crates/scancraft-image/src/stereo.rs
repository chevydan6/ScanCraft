//! Paired rectification maps retain their exact source/output geometry.
use crate::{GrayImage, MaskedImage, UndistortMap};
use scancraft_calibration::rectification::StereoRectification;
use scancraft_geometry::work::WorkControl;

pub struct StereoImageMaps {
    rectification: StereoRectification,
    left: UndistortMap,
    right: UndistortMap,
}
impl StereoImageMaps {
    pub fn build(rectification: StereoRectification, work: &WorkControl) -> Result<Self, String> {
        let source = rectification.source();
        let output = rectification.geometry();
        let left = UndistortMap::build(
            &source.left,
            &output.left,
            rectification.left_from_rectified().rotation,
            work,
        )?;
        let right = UndistortMap::build(
            &source.right,
            &output.right,
            rectification.right_from_rectified().rotation,
            work,
        )?;
        Ok(Self {
            rectification,
            left,
            right,
        })
    }
    pub fn rectification(&self) -> &StereoRectification {
        &self.rectification
    }
    /// Return neither image if one fails. Does not establish capture timing.
    pub fn apply(
        &self,
        left: GrayImage<'_>,
        right: GrayImage<'_>,
        work: &WorkControl,
    ) -> Result<(MaskedImage, MaskedImage), String> {
        let left = self.left.apply(left, work)?;
        let right = self.right.apply(right, work)?;
        Ok((left, right))
    }
}

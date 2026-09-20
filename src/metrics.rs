//! Host-supplied text metrics.
//!
//! Text measurement normally resolves faces from the system font database,
//! which ties layout to whatever faces a machine happens to have installed. A
//! host that already owns a font collection — an editor, a document pipeline,
//! an export that pins its fonts — can install a [`TextMetrics`] on
//! [`LayoutConfig`](crate::LayoutConfig) instead, so that labels are measured
//! with the same faces they are drawn with.
//!
//! Every method returns `None` when the host cannot serve the request, and the
//! renderer then measures with its own database and estimates. Installing a
//! provider therefore never removes a fallback, it only puts the host's faces
//! first. The renderer's own PNG output keeps using its own font database.

/// Text measurement a host supplies in place of the system font database.
pub trait TextMetrics: std::fmt::Debug + Send + Sync {
    /// The advance width of `text` at `font_size`, in the first family of the
    /// comma-separated `font_family` list the host can draw it with.
    ///
    /// The list is the theme's own `font_family`, in priority order. `None`
    /// hands the request back to the renderer's system-font measurement.
    fn measure_text_width(&self, text: &str, font_size: f32, font_family: &str) -> Option<f32>;

    /// The average advance used to cap a label's width. The default measures
    /// a Latin sample through [`Self::measure_text_width`].
    fn average_char_width(&self, font_family: &str, font_size: f32) -> Option<f32> {
        const SAMPLE: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let width = self.measure_text_width(SAMPLE, font_size, font_family)?;
        Some(width / SAMPLE.chars().count().max(1) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Fixed;

    impl TextMetrics for Fixed {
        fn measure_text_width(
            &self,
            text: &str,
            font_size: f32,
            _font_family: &str,
        ) -> Option<f32> {
            Some(text.chars().count() as f32 * font_size)
        }
    }

    #[test]
    fn the_default_average_measures_a_latin_sample() {
        let average = Fixed.average_char_width("serif", 10.0).unwrap();
        assert!((average - 10.0).abs() < 1e-3, "{average}");
    }

    #[test]
    fn a_provider_is_a_trait_object_a_config_can_hold() {
        let provider: std::sync::Arc<dyn TextMetrics> = std::sync::Arc::new(Fixed);
        assert_eq!(provider.measure_text_width("ab", 10.0, "serif"), Some(20.0));
    }
}

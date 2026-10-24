use std::{
    borrow::Cow,
    sync::{Arc, Mutex, Weak},
};

use cap_project::ProjectConfiguration;

#[derive(Default)]
pub(crate) struct StyleProjectCache {
    entry: Mutex<Option<Entry>>,
}

struct Entry {
    source: Weak<ProjectConfiguration>,
    start: f64,
    end: f64,
    resolved: Arc<ProjectConfiguration>,
}

impl StyleProjectCache {
    pub(crate) fn resolve(
        &self,
        project: &Arc<ProjectConfiguration>,
        time: f64,
    ) -> Arc<ProjectConfiguration> {
        let mut entry = self.entry.lock().unwrap_or_else(|error| error.into_inner());
        let styles = project
            .timeline
            .as_ref()
            .map(|timeline| timeline.style_segments.as_slice())
            .unwrap_or_default();
        if styles.is_empty() || !time.is_finite() {
            *entry = None;
            return Arc::clone(project);
        }

        if let Some(cached) = entry.as_ref()
            && cached.source.as_ptr() == Arc::as_ptr(project)
            && time >= cached.start
            && time < cached.end
        {
            return Arc::clone(&cached.resolved);
        }

        // style_at only changes at segment boundaries; layer animations still sample each frame.
        let mut start = f64::NEG_INFINITY;
        let mut end = f64::INFINITY;
        for segment in styles {
            if !segment.is_active_at(segment.start) {
                continue;
            }
            for boundary in [segment.start, segment.end] {
                if boundary <= time {
                    start = start.max(boundary);
                } else {
                    end = end.min(boundary);
                }
            }
        }

        let resolved = match project.style_at(time) {
            Cow::Borrowed(_) => Arc::clone(project),
            Cow::Owned(styled) => Arc::new(styled),
        };
        // Retaining a Weak prevents address reuse and makes Arc::make_mut
        // disassociate the source before an edit can invalidate this interval.
        *entry = Some(Entry {
            source: Arc::downgrade(project),
            start,
            end,
            resolved: Arc::clone(&resolved),
        });
        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_project::{BackgroundConfiguration, Camera, CursorConfiguration, StyleSegment};

    fn project(styles: Vec<StyleSegment>) -> Arc<ProjectConfiguration> {
        let mut timeline: cap_project::TimelineConfiguration =
            serde_json::from_value(serde_json::json!({
                "segments": [],
                "zoomSegments": [],
            }))
            .unwrap();
        timeline.style_segments = styles;
        Arc::new(ProjectConfiguration {
            timeline: Some(timeline),
            ..Default::default()
        })
    }

    fn style(start: f64, end: f64, padding: f64) -> StyleSegment {
        StyleSegment {
            start,
            end,
            overrides: cap_project::StyleOverrides {
                background: Some(BackgroundConfiguration {
                    padding,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn assert_resolved(
        cache: &StyleProjectCache,
        project: &Arc<ProjectConfiguration>,
        time: f64,
    ) -> Arc<ProjectConfiguration> {
        let resolved = cache.resolve(project, time);
        let expected = project.style_at(time);
        assert_eq!(
            serde_json::to_value(resolved.as_ref()).unwrap(),
            serde_json::to_value(expected.as_ref()).unwrap(),
            "time={time}"
        );
        assert_eq!(resolved.hidden_text_segments, expected.hidden_text_segments);
        resolved
    }

    #[test]
    fn intervals_and_seeks_match_the_uncached_resolver() {
        let mut camera = style(3.0, 6.0, 12.0);
        camera.track = 2;
        camera.overrides.camera = Some(Camera {
            hide: true,
            ..Default::default()
        });
        let mut cursor = StyleSegment {
            start: 4.0,
            end: 7.0,
            track: 3,
            ..Default::default()
        };
        let mut cursor_configuration = CursorConfiguration::default();
        cursor_configuration.raw = true;
        cursor.overrides.cursor = Some(cursor_configuration);
        let mut tied = style(3.0, 5.0, 42.0);
        tied.track = 2;
        let mut disabled = style(1.0, 8.0, 99.0);
        disabled.enabled = false;
        let project = project(vec![
            style(0.0, 10.0, 20.0),
            camera,
            cursor,
            tied,
            disabled,
            style(2.0, 2.0, 99.0),
        ]);
        let cache = StyleProjectCache::default();
        let first = assert_resolved(&cache, &project, 0.0);
        assert!(!Arc::ptr_eq(&first, &project));
        assert!(Arc::ptr_eq(&first, &cache.resolve(&project, 2.0)));
        for time in [
            -1.0,
            0.0,
            2.99,
            3.0,
            3.5,
            4.0,
            5.0,
            6.0,
            7.0,
            10.0,
            8.0,
            4.5,
            3.0,
            f64::NAN,
            2.0,
            f64::INFINITY,
            3.0,
            f64::NEG_INFINITY,
        ] {
            assert_resolved(&cache, &project, time);
        }
    }

    #[test]
    fn source_edits_and_returned_snapshots_stay_isolated() {
        let mut project = project(vec![style(0.0, 10.0, 20.0)]);
        let cache = StyleProjectCache::default();
        let first = assert_resolved(&cache, &project, 2.0);
        assert_eq!(Arc::strong_count(&project), 1);
        let before_edit = Arc::downgrade(&project);
        let edited = Arc::make_mut(&mut project);
        edited.timeline.as_mut().unwrap().style_segments[0] = style(0.0, 10.0, 40.0);
        edited.audio.mute = true;
        edited.hidden_text_segments.push(7);
        assert!(before_edit.upgrade().is_none());
        let mut second = assert_resolved(&cache, &project, 2.0);
        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(first.background.padding, 20.0);
        assert_eq!(second.background.padding, 40.0);
        Arc::make_mut(&mut second).background.padding = 75.0;
        let cached = assert_resolved(&cache, &project, 2.0);
        assert!(!Arc::ptr_eq(&second, &cached));
        assert_eq!(cached.background.padding, 40.0);
    }

    #[test]
    fn removing_styles_returns_the_source_and_clears_the_cache() {
        let mut project = project(vec![style(0.0, 10.0, 20.0)]);
        let cache = StyleProjectCache::default();
        let resolved = Arc::downgrade(&cache.resolve(&project, 2.0));
        Arc::make_mut(&mut project)
            .timeline
            .as_mut()
            .unwrap()
            .style_segments
            .clear();
        assert!(Arc::ptr_eq(
            &project,
            &assert_resolved(&cache, &project, 2.0)
        ));
        assert!(resolved.upgrade().is_none());
        assert!(cache.entry.lock().unwrap().is_none());
        Arc::make_mut(&mut project).timeline = None;
        assert!(Arc::ptr_eq(
            &project,
            &assert_resolved(&cache, &project, 2.0)
        ));
    }

    #[test]
    fn active_style_cache_keeps_one_snapshot_and_releases_source() {
        let project = project(vec![style(0.0, 4.0, 20.0), style(4.0, 8.0, 40.0)]);
        let cache = StyleProjectCache::default();
        let source = Arc::downgrade(&project);
        let first = Arc::downgrade(&cache.resolve(&project, 2.0));
        let second = Arc::downgrade(&cache.resolve(&project, 5.0));
        assert!(first.upgrade().is_none());
        drop(project);
        assert!(source.upgrade().is_none());
        assert!(second.upgrade().is_some());
        drop(cache);
        assert!(second.upgrade().is_none());
    }
}

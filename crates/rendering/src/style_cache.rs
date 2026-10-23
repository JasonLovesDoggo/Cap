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
    fn repeated_frames_share_one_resolved_configuration() {
        let project = project(vec![style(2.0, 8.0, 30.0)]);
        let cache = StyleProjectCache::default();
        let first = assert_resolved(&cache, &project, 2.0);
        assert!(!Arc::ptr_eq(&first, &project));
        for time in [2.1, 5.0, 7.999, 3.0, 2.0] {
            assert!(Arc::ptr_eq(
                &first,
                &assert_resolved(&cache, &project, time)
            ));
        }
        for time in [8.0, 1.999, -1.0, 100.0] {
            assert!(Arc::ptr_eq(
                &project,
                &assert_resolved(&cache, &project, time)
            ));
        }
    }

    #[test]
    fn overlaps_and_seeks_preserve_property_precedence_and_boundaries() {
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
        let project = project(vec![style(-0.0, 10.0, 20.0), camera, cursor, tied]);
        let cache = StyleProjectCache::default();
        for time in [
            -1.0, -0.0, 0.0, 2.99, 3.0, 3.5, 4.0, 5.0, 6.0, 7.0, 10.0, 8.0, 4.5, 3.0, 0.0, 10.0,
        ] {
            assert_resolved(&cache, &project, time);
        }
    }

    #[test]
    fn invalid_segments_and_nonfinite_times_match_uncached_resolution() {
        let mut disabled = style(1.0, 6.0, 50.0);
        disabled.enabled = false;
        let project = project(vec![
            style(2.0, 4.0, 25.0),
            disabled,
            style(-1.0, 6.0, 60.0),
            style(2.0, 2.0, 60.0),
            style(4.0, 1.0, 60.0),
            style(f64::NAN, 6.0, 60.0),
            style(1.0, f64::INFINITY, 60.0),
            style(f64::NEG_INFINITY, 6.0, 60.0),
        ]);
        let cache = StyleProjectCache::default();
        for time in [
            2.5,
            f64::NAN,
            3.0,
            f64::INFINITY,
            2.0,
            f64::NEG_INFINITY,
            -2.0,
            0.0,
            4.0,
            100.0,
        ] {
            assert_resolved(&cache, &project, time);
        }
    }

    #[test]
    fn editing_a_uniquely_owned_source_invalidates_the_weak_identity() {
        let mut project = project(vec![style(0.0, 10.0, 20.0)]);
        let cache = StyleProjectCache::default();
        let first = assert_resolved(&cache, &project, 2.0);
        assert_eq!(Arc::strong_count(&project), 1);
        assert!(Arc::get_mut(&mut project).is_none());
        let before_edit = Arc::downgrade(&project);
        let edited = Arc::make_mut(&mut project);
        edited.timeline.as_mut().unwrap().style_segments[0]
            .overrides
            .background
            .as_mut()
            .unwrap()
            .padding = 40.0;
        edited.hidden_text_segments.push(7);
        assert!(before_edit.upgrade().is_none());
        let second = assert_resolved(&cache, &project, 2.0);
        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(first.background.padding, 20.0);
        assert_eq!(second.background.padding, 40.0);
    }

    #[test]
    fn editing_returned_uniforms_cannot_mutate_the_cached_snapshot() {
        let project = project(vec![style(0.0, 10.0, 20.0)]);
        let cache = StyleProjectCache::default();
        let mut resolved = cache.resolve(&project, 2.0);
        Arc::make_mut(&mut resolved).background.padding = 75.0;
        let cached = assert_resolved(&cache, &project, 2.0);
        assert!(!Arc::ptr_eq(&resolved, &cached));
        assert_eq!(resolved.background.padding, 75.0);
        assert_eq!(cached.background.padding, 20.0);
    }

    #[test]
    fn replacing_or_removing_styles_and_unrelated_edits_refresh_the_snapshot() {
        let mut project = project(vec![style(0.0, 10.0, 20.0)]);
        let cache = StyleProjectCache::default();
        assert_resolved(&cache, &project, 2.0);
        let retained_source = Arc::clone(&project);
        Arc::make_mut(&mut project).audio.mute = true;
        assert_resolved(&cache, &project, 2.0);
        assert_resolved(&cache, &retained_source, 2.0);
        let mut replacement = project.as_ref().clone();
        replacement.timeline.as_mut().unwrap().style_segments = vec![style(1.0, 4.0, 50.0)];
        project = Arc::new(replacement);
        assert_resolved(&cache, &project, 2.0);
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
        assert!(cache.entry.lock().unwrap().is_none());
        Arc::make_mut(&mut project).timeline = None;
        assert!(Arc::ptr_eq(
            &project,
            &assert_resolved(&cache, &project, 2.0)
        ));
    }

    #[test]
    fn cache_retains_only_one_resolved_project() {
        let project = project(vec![style(0.0, 4.0, 20.0), style(4.0, 8.0, 40.0)]);
        let cache = StyleProjectCache::default();
        let first = cache.resolve(&project, 2.0);
        let first_weak = Arc::downgrade(&first);
        drop(first);
        let second = cache.resolve(&project, 5.0);
        assert!(first_weak.upgrade().is_none());
        let second_weak = Arc::downgrade(&second);
        drop(second);
        drop(cache);
        assert!(second_weak.upgrade().is_none());
    }

    #[test]
    fn concurrent_readers_share_a_single_snapshot() {
        let project = project(vec![style(0.0, 10.0, 20.0)]);
        let cache = StyleProjectCache::default();
        let barrier = std::sync::Barrier::new(8);
        let resolved = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        assert_resolved(&cache, &project, 2.0)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(
            resolved
                .iter()
                .all(|value| Arc::ptr_eq(&resolved[0], value))
        );
    }

    #[test]
    fn poisoned_cache_remains_usable() {
        let project = project(vec![style(0.0, 10.0, 20.0)]);
        let cache = StyleProjectCache::default();
        assert_resolved(&cache, &project, 2.0);
        let poisoned = std::panic::catch_unwind(|| {
            let _guard = cache.entry.lock().unwrap();
            panic!("poison the cache");
        });
        assert!(poisoned.is_err());
        assert_resolved(&cache, &project, 2.0);
        assert_resolved(&cache, &project, 12.0);
    }
}

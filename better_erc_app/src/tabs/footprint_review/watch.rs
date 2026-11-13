//! The files the review shows, watched on disk (VIEW-31): the symbol list's (the board, its schematics, the
//! library tables, the symbol libraries, the footprint files), the 3D models', the datasheet's. A change reloads
//! what it touches, the view kept.
//!
//! The watcher (`notify`, inotify on Linux) watches the directories the files are in, so a file saved by writing a
//! new one and renaming it over the old (KiCad, most editors) is seen too; its events only say "look again". What
//! decides is each file's stamp (its length, modification time and a hash of its content, or of the names in a
//! directory): a file whose stamp is the same as when it was read did not change. So a file the tab wrote itself
//! (Accept) is stamped again after the write ([`Watch::restamp`]) and its event reloads nothing, and the same check
//! runs before a symbol is picked ([`Watch::changed`]), events or not.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant, SystemTime};

/// How long the files must stay quiet after an event before they are looked at: a save is often several writes
pub(super) const DEBOUNCE: Duration = Duration::from_millis(300);

/// What a file's change reloads
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum Group {
    /// The symbol list, the board, the footprint shown and the symbol's fields
    List,
    /// The footprint's 3D models
    Model,
    /// The datasheet open
    Pdf,
}

impl Group {
    pub(super) fn name(self) -> &'static str {
        match self {
            Group::List => "the symbols and footprints",
            Group::Model => "the 3D model",
            Group::Pdf => "the datasheet",
        }
    }
}

/// What a file was when it was read; `None` in [`Watched::stamp`]: it was not there
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
    hash: u64,
}

/// A file or directory as it is now: its content's hash (a directory: its names'), `None` when it is not there
fn stamp(path: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    let mut h = std::hash::DefaultHasher::new();
    if meta.is_dir() {
        let mut names: Vec<_> = std::fs::read_dir(path)
            .ok()?
            .filter_map(|e| Some(e.ok()?.file_name()))
            .collect();
        names.sort();
        names.hash(&mut h);
    } else {
        std::fs::read(path).ok()?.hash(&mut h);
    }
    Some(Stamp {
        len: meta.len(),
        modified: meta.modified().ok(),
        hash: h.finish(),
    })
}

/// `path` looked at again against `was`: `None` when its length and time are the same (not read again), else its
/// stamp now and whether its content differs (a file written again the same is no change)
fn look(path: &Path, was: &Option<Stamp>) -> Option<(Option<Stamp>, bool)> {
    let meta = std::fs::metadata(path).ok();
    match (was, &meta) {
        (None, None) => return None,
        (Some(w), Some(m)) if w.len == m.len() && w.modified == m.modified().ok() => return None,
        _ => {}
    }
    let now = stamp(path);
    let differs = match (was, &now) {
        (Some(a), Some(b)) => a.len != b.len || a.hash != b.hash,
        (a, b) => a.is_some() != b.is_some(),
    };
    Some((now, differs))
}

struct Watched {
    group: Group,
    stamp: Option<Stamp>,
}

/// The files watched, their stamps, the watcher and the events waiting
#[derive(Default)]
pub(super) struct Watch {
    files: HashMap<PathBuf, Watched>,
    /// The directories the watcher watches
    dirs: HashSet<PathBuf>,
    watcher: Option<notify::RecommendedWatcher>,
    events: Option<Receiver<Vec<PathBuf>>>,
    /// An event for a watched file came in: the files are looked at once they stayed quiet until then
    due: Option<Instant>,
    /// Why the watcher could not start, once (the Reload button still works)
    pub(super) error: Option<String>,
}

impl Watch {
    /// Starts the watcher; its events wake `wake`'s UI when given (not in tests, which ask for each frame)
    pub(super) fn start(&mut self, wake: Option<egui::Context>) {
        if self.watcher.is_some() {
            return;
        }
        let (tx, rx): (Sender<Vec<PathBuf>>, _) = channel();
        let handler = move |ev: notify::Result<notify::Event>| {
            if let Ok(ev) = ev
                && !matches!(ev.kind, notify::EventKind::Access(_))
                && tx.send(ev.paths).is_ok()
                && let Some(ctx) = &wake
            {
                ctx.request_repaint();
            }
        };
        match notify::recommended_watcher(handler) {
            Ok(w) => {
                self.watcher = Some(w);
                self.events = Some(rx);
                self.dirs.clear();
                self.watch_dirs();
            }
            Err(e) => {
                self.error = Some(format!(
                    "files are not watched ({e}): Reload reads them again"
                ))
            }
        }
    }

    /// The files of `group` are `paths` from now on. A file new to it is stamped as it is now (just read); one it
    /// had keeps its stamp, so a change not looked at yet is still found (and a symbol picked doesn't read every
    /// library again for its stamp)
    pub(super) fn set(&mut self, group: Group, paths: impl IntoIterator<Item = PathBuf>) {
        let (mut had, others): (HashMap<_, _>, HashMap<_, _>) = std::mem::take(&mut self.files)
            .into_iter()
            .partition(|(_, w)| w.group == group);
        self.files = others;
        for p in paths {
            let w = had.remove(&p).unwrap_or_else(|| Watched {
                group,
                stamp: stamp(&p),
            });
            self.files.entry(p).or_insert(w);
        }
        self.watch_dirs();
    }

    /// `paths` were written by the tab itself: what they are now is what it knows
    pub(super) fn restamp<'a>(&mut self, paths: impl IntoIterator<Item = &'a Path>) {
        for p in paths {
            if let Some(w) = self.files.get_mut(p) {
                w.stamp = stamp(p);
            }
        }
    }

    /// The files watched, sorted (tests)
    #[cfg(test)]
    pub(super) fn files(&self, group: Group) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|(_, w)| w.group == group)
            .map(|(p, _)| p.clone())
            .collect();
        v.sort();
        v
    }

    /// The directories to watch: each file's own, and a directory watched as a file (a `.kicad_symdir`) itself
    fn watch_dirs(&mut self) {
        let Some(w) = &mut self.watcher else { return };
        use notify::Watcher;
        let mut want: HashSet<PathBuf> = HashSet::new();
        for p in self.files.keys() {
            if p.is_dir() {
                want.insert(p.clone());
            }
            if let Some(d) = p.parent().filter(|d| d.is_dir()) {
                want.insert(d.to_path_buf());
            }
        }
        for d in self.dirs.difference(&want) {
            let _ = w.unwatch(d);
        }
        let mut dirs = HashSet::new();
        for d in want {
            if self.dirs.contains(&d) || w.watch(&d, notify::RecursiveMode::NonRecursive).is_ok() {
                dirs.insert(d);
            }
        }
        self.dirs = dirs;
    }

    /// Whether an event names a watched file, or one in a watched directory (a symbol file in a `.kicad_symdir`)
    fn concerns(&self, p: &Path) -> bool {
        self.files.contains_key(p) || p.parent().is_some_and(|d| self.files.contains_key(d))
    }

    /// Takes the watcher's events in. Returns the groups whose files changed, once the files stayed quiet for
    /// [`DEBOUNCE`] after the last event; while waiting, how long until it looks (the UI's next frame)
    pub(super) fn poll(&mut self, now: Instant) -> (Vec<Group>, Option<Duration>) {
        if let Some(rx) = &self.events {
            let mut hit = false;
            while let Ok(paths) = rx.try_recv() {
                hit |= paths.iter().any(|p| self.concerns(p));
            }
            if hit {
                self.due = Some(now + DEBOUNCE);
            }
        }
        match self.due {
            Some(due) if now >= due => {
                self.due = None;
                (self.changed(), None)
            }
            Some(due) => (Vec::new(), Some(due - now)),
            None => (Vec::new(), None),
        }
    }

    /// The groups whose files changed since they were stamped, each file stamped anew. Changed files' groups
    /// are in a fixed order (the list first: it may bring another model and datasheet)
    pub(super) fn changed(&mut self) -> Vec<Group> {
        let mut groups: Vec<Group> = Vec::new();
        for (p, w) in &mut self.files {
            let Some((now, differs)) = look(p, &w.stamp) else {
                continue;
            };
            w.stamp = now;
            if differs && !groups.contains(&w.group) {
                groups.push(w.group);
            }
        }
        groups.sort();
        groups
    }

    /// Every file stamped as it is now: after a reload of everything
    pub(super) fn restamp_all(&mut self) {
        for (p, w) in &mut self.files {
            w.stamp = stamp(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A change of content is a change; the same content written again is none, nor a file the tab wrote itself;
    /// a file that comes or goes is one; a directory's change is a file added or taken away
    #[test]
    fn stamps_decide() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.kicad_mod");
        let later = dir.path().join("later.kicad_mod");
        let symdir = dir.path().join("Lib.kicad_symdir");
        std::fs::write(&a, "one").unwrap();
        std::fs::create_dir(&symdir).unwrap();
        let mut w = Watch::default();
        w.set(Group::List, [a.clone(), later.clone(), symdir.clone()]);
        w.set(Group::Pdf, [dir.path().join("d.pdf")]);
        assert_eq!(w.changed(), []);

        std::fs::write(&a, "two, longer").unwrap();
        assert_eq!(w.changed(), [Group::List]);
        assert_eq!(w.changed(), [], "stamped anew");

        // The same bytes again (a save without a change): its time moves, its content does not
        std::fs::write(&a, "two, longer").unwrap();
        assert_eq!(w.changed(), []);

        // Written by the tab and stamped again: no change
        std::fs::write(&a, "three, by Accept").unwrap();
        w.restamp([a.as_path()]);
        assert_eq!(w.changed(), []);

        std::fs::write(&later, "now there").unwrap();
        std::fs::write(dir.path().join("d.pdf"), "%PDF").unwrap();
        assert_eq!(w.changed(), [Group::List, Group::Pdf]);

        std::fs::write(symdir.join("R.kicad_sym"), "(kicad_symbol_lib)").unwrap();
        assert_eq!(w.changed(), [Group::List]);
        std::fs::remove_file(&later).unwrap();
        assert_eq!(w.changed(), [Group::List]);

        // A group's files given again replace its old ones; one it had keeps its stamp, a change not looked at yet
        // is still found
        std::fs::write(&a, "four, not looked at yet").unwrap();
        w.set(Group::List, [a.clone()]);
        assert_eq!(w.changed(), [Group::List]);
        assert_eq!(w.files(Group::List), std::slice::from_ref(&a));
        std::fs::write(symdir.join("C.kicad_sym"), "(kicad_symbol_lib)").unwrap();
        assert_eq!(w.changed(), []);
    }

    /// The watcher's events lead to a look once the files stayed quiet: a change of a watched file is found, one
    /// of a file not watched beside it is not. Waits for the events (a condition), bounded so a lost one fails
    #[test]
    fn events_after_quiet() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.kicad_mod");
        std::fs::write(&a, "one").unwrap();
        let mut w = Watch::default();
        w.start(None);
        assert!(w.error.is_none(), "{:?}", w.error);
        w.set(Group::List, [a.clone()]);

        // The debounce's time is the test's: each look is a quiet time later than the last
        let mut t = Instant::now();
        let mut wait = |w: &mut Watch| {
            let until = Instant::now() + Duration::from_secs(20);
            loop {
                t += DEBOUNCE * 2;
                let (groups, _) = w.poll(t);
                if !groups.is_empty() || Instant::now() > until {
                    return groups;
                }
                std::thread::yield_now();
            }
        };
        std::fs::write(dir.path().join("other.txt"), "not watched").unwrap();
        std::fs::write(&a, "two, longer").unwrap();
        assert_eq!(wait(&mut w), [Group::List]);

        // Saved the way KiCad saves: a new file renamed over the old
        let tmp = dir.path().join("a.kicad_mod.tmp");
        std::fs::write(&tmp, "three, by rename").unwrap();
        std::fs::rename(&tmp, &a).unwrap();
        assert_eq!(wait(&mut w), [Group::List]);
    }

    /// Before the quiet time is over nothing is looked at, and the time left is said for the next frame
    #[test]
    fn debounced() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.kicad_mod");
        std::fs::write(&a, "one").unwrap();
        let mut w = Watch::default();
        let (tx, rx) = channel();
        w.events = Some(rx);
        w.set(Group::List, [a.clone()]);
        std::fs::write(&a, "two, longer").unwrap();
        tx.send(vec![a.clone()]).unwrap();
        let t0 = Instant::now();
        let (groups, wait) = w.poll(t0);
        assert_eq!(groups, []);
        assert_eq!(wait, Some(DEBOUNCE));
        // Another event in the meantime starts the quiet time again
        tx.send(vec![a.clone()]).unwrap();
        let (groups, wait) = w.poll(t0 + DEBOUNCE / 2);
        assert_eq!(groups, []);
        assert_eq!(wait, Some(DEBOUNCE));
        assert_eq!(w.poll(t0 + DEBOUNCE * 2).0, [Group::List]);
        // An event for a file not watched starts nothing
        tx.send(vec![dir.path().join("other")]).unwrap();
        assert_eq!(w.poll(t0 + DEBOUNCE * 3), (vec![], None));
    }
}

# Performance

About keeping Mado fast and light: the three aims of a pass, how one is done, and the log that makes each pass start where the last one stopped.

## Rules

- **A pass has three aims:** the best speed; the least use of the computer (CPU, memory, power, wakeups, disk, network, processes); and nothing the app does lost. The third is never traded for the other two.
- **Measured before and after,** with a stopwatch in a copy or a throwaway test, taken out when done (`docs/probing.md`, Measuring). The numbers go in the log.
- **Incremental.** A pass reads the log's last entry and looks only at what changed since the commit it names, plus anything it left for later. With no entry, the pass covers everything.
- **A row's call runs once a row a frame,** and a lookup made at every frame is answered from memory: the app's own writes go into the memory as they go to the disk, and the disk is looked at no more than every half second for another process's write.
- **A path is a name, not a look at the disk.** Whatever the path's root once did to the disk is done once a run.

## A pass

1. Read the log's last entry, or with none, take everything.
2. List what is new since its commit.
3. Find the hot paths: what `render` and each row call, clocks that wake with nothing to move, waits on the main thread, loops and what each turn costs, what grows with use.
4. Time each suspect.
5. Fix the cause where every caller goes through.
6. Time it again.
7. Check nothing is lost: `cargo test -p mado-core -p mado-app`, and a probe copy of each part changed.
8. Write the log entry: version and date, the commits covered, what was looked at, what was found with numbers before and after, what was left as it is and why, what was left for later.

## Log

Newest first. No pass yet: the first covers everything.

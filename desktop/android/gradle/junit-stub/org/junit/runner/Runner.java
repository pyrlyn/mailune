package org.junit.runner;

import org.junit.runner.notification.RunNotifier;

/** Runs one test class. */
public abstract class Runner {
    public abstract Description getDescription();

    public abstract void run(RunNotifier notifier);
}

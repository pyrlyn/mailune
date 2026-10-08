package org.junit.runner.manipulation;

import org.junit.runner.Description;

/** A filter the worker may apply. This build does not select tests by name. */
public abstract class Filter {
    public abstract boolean shouldRun(Description description);
}

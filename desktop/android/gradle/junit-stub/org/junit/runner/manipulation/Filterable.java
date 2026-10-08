package org.junit.runner.manipulation;

/** A runner that can drop tests. */
public interface Filterable {
    void filter(Filter filter) throws NoTestsRemainException;
}

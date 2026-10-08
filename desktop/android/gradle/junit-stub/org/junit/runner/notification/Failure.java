package org.junit.runner.notification;

import org.junit.runner.Description;

/** A test method that threw. */
public final class Failure {
    private final Description description;
    private final Throwable exception;

    public Failure(Description description, Throwable exception) {
        this.description = description;
        this.exception = exception;
    }

    public Description getDescription() {
        return description;
    }

    public Throwable getException() {
        return exception;
    }
}

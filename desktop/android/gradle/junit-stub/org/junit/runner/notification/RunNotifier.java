package org.junit.runner.notification;

import java.util.List;

import org.junit.runner.Description;
import org.junit.runner.Result;

/** Delivers events to every listener. */
public final class RunNotifier {
    private final List<RunListener> listeners;

    public RunNotifier(List<RunListener> listeners) {
        this.listeners = listeners;
    }

    public void fireTestRunStarted(Description description) {
        each(listener -> listener.testRunStarted(description));
    }

    public void fireTestRunFinished(Result result) {
        each(listener -> listener.testRunFinished(result));
    }

    public void fireTestSuiteStarted(Description description) {
        each(listener -> listener.testSuiteStarted(description));
    }

    public void fireTestSuiteFinished(Description description) {
        each(listener -> listener.testSuiteFinished(description));
    }

    public void fireTestStarted(Description description) {
        each(listener -> listener.testStarted(description));
    }

    public void fireTestFinished(Description description) {
        each(listener -> listener.testFinished(description));
    }

    public void fireTestFailure(Failure failure) {
        each(listener -> listener.testFailure(failure));
    }

    private void each(Event event) {
        for (RunListener listener : listeners) {
            try {
                event.fire(listener);
            } catch (Exception exception) {
                throw new RuntimeException(exception);
            }
        }
    }

    private interface Event {
        void fire(RunListener listener) throws Exception;
    }
}

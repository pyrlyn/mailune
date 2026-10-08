package org.junit.runner;

import java.util.ArrayList;
import java.util.List;

import org.junit.runner.notification.RunListener;
import org.junit.runner.notification.RunNotifier;

/** Entry the Gradle worker calls. */
public final class JUnitCore {
    private final List<RunListener> listeners = new ArrayList<RunListener>();

    public void addListener(RunListener listener) {
        listeners.add(listener);
    }

    public Result run(Request request) {
        Runner runner = request.getRunner();
        RunNotifier notifier = new RunNotifier(listeners);
        notifier.fireTestRunStarted(runner.getDescription());
        runner.run(notifier);
        Result result = new Result();
        notifier.fireTestRunFinished(result);
        return result;
    }
}

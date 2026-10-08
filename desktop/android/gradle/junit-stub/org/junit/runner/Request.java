package org.junit.runner;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.List;

import org.junit.Test;
import org.junit.runner.notification.Failure;
import org.junit.runner.notification.RunNotifier;

/** Builds a runner for one test class. */
public abstract class Request {
    public abstract Runner getRunner();

    public static Request aClass(final Class<?> testClass) {
        return new Request() {
            @Override
            public Runner getRunner() {
                return new ClassRunner(testClass);
            }
        };
    }

    private static final class ClassRunner extends Runner {
        private final Class<?> testClass;
        private final List<Method> methods = new ArrayList<Method>();
        private final Description description;

        ClassRunner(Class<?> testClass) {
            this.testClass = testClass;
            for (Method method : testClass.getMethods()) {
                if (method.getAnnotation(Test.class) != null && method.getParameterCount() == 0) {
                    methods.add(method);
                }
            }
            description = Description.createSuiteDescription(testClass);
            for (Method method : methods) {
                description.addChild(Description.createTestDescription(testClass, method.getName()));
            }
        }

        @Override
        public Description getDescription() {
            return description;
        }

        @Override
        public void run(RunNotifier notifier) {
            notifier.fireTestSuiteStarted(description);
            for (Method method : methods) {
                Description child = Description.createTestDescription(testClass, method.getName());
                notifier.fireTestStarted(child);
                try {
                    Object instance = testClass.getDeclaredConstructor().newInstance();
                    method.invoke(instance);
                } catch (InvocationTargetException exception) {
                    Throwable cause = exception.getCause() == null ? exception : exception.getCause();
                    notifier.fireTestFailure(new Failure(child, cause));
                } catch (Throwable exception) {
                    notifier.fireTestFailure(new Failure(child, exception));
                }
                notifier.fireTestFinished(child);
            }
            notifier.fireTestSuiteFinished(description);
        }
    }
}

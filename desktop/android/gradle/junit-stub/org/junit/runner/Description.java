package org.junit.runner;

import java.lang.annotation.Annotation;
import java.util.ArrayList;

/** Names a suite or a test method for the Gradle test worker. */
public final class Description {
    public static final Description EMPTY = new Description(null, "No Tests");
    public static final Description TEST_MECHANISM = new Description(null, "Test mechanism");

    private final Class<?> testClass;
    private final String displayName;
    private final ArrayList<Description> children = new ArrayList<Description>();

    private Description(Class<?> testClass, String displayName) {
        this.testClass = testClass;
        this.displayName = displayName;
    }

    public static Description createSuiteDescription(Class<?> testClass) {
        return new Description(testClass, testClass.getName());
    }

    public static Description createSuiteDescription(String name, Annotation... annotations) {
        return new Description(null, name);
    }

    public static Description createTestDescription(Class<?> testClass, String name) {
        return new Description(testClass, name + "(" + testClass.getName() + ")");
    }

    public static Description createTestDescription(String className, String name, Annotation... annotations) {
        return new Description(null, name + "(" + className + ")");
    }

    public void addChild(Description child) {
        children.add(child);
    }

    public ArrayList<Description> getChildren() {
        return children;
    }

    public boolean isSuite() {
        return !children.isEmpty();
    }

    public boolean isTest() {
        return children.isEmpty();
    }

    public String getDisplayName() {
        return displayName;
    }

    public String getClassName() {
        if (testClass != null) {
            return testClass.getName();
        }
        int open = displayName.lastIndexOf('(');
        int close = displayName.lastIndexOf(')');
        if (open >= 0 && close > open) {
            return displayName.substring(open + 1, close);
        }
        return displayName;
    }

    public String getMethodName() {
        int open = displayName.lastIndexOf('(');
        if (open > 0) {
            return displayName.substring(0, open);
        }
        return null;
    }

    public Class<?> getTestClass() {
        return testClass;
    }

    @Override
    public String toString() {
        return displayName;
    }

    @Override
    public boolean equals(Object other) {
        if (!(other instanceof Description)) {
            return false;
        }
        return displayName.equals(((Description) other).displayName);
    }

    @Override
    public int hashCode() {
        return displayName.hashCode();
    }
}

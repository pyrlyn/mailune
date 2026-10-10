# The Vala side of mailune-capi: tests/capi.vala, built against
# vapi/mailune.vapi and the library cargo built, and run as a Meson test:
#   cargo build -p mailune-capi
#   meson setup build -Dcapi_dir=$PWD/../../target/debug && meson test -C build
project('mailune-capi', 'vala', 'c', meson_version: '>= 1.0')

capi_dir = get_option('capi_dir')
if capi_dir == ''
  error('pass -Dcapi_dir=<cargo target dir>/debug, where libmailune_capi was built')
endif

mailune = declare_dependency(
  dependencies: [
    meson.get_compiler('vala').find_library('mailune', dirs: meson.current_source_dir() / 'vapi'),
    meson.get_compiler('c').find_library('mailune_capi', dirs: capi_dir),
  ],
  include_directories: include_directories('include'),
)

capi_test = executable(
  'capi-test',
  'tests/capi.vala',
  dependencies: [
    dependency('glib-2.0'),
    dependency('gobject-2.0'),
    dependency('json-glib-1.0'),
    mailune,
  ],
)

test('capi', capi_test, env: {'LD_LIBRARY_PATH': capi_dir, 'DYLD_LIBRARY_PATH': capi_dir}, timeout: 60)

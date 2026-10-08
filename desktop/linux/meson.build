# C library and link test for the mailune-capi headers.
# shell.vala is present and is not compiled: the GTK target stays off.
project('mailune-linux', 'c', meson_version: '>= 1.0', default_options: ['c_std=c11', 'warning_level=2'])

if get_option('gtk')
  error('GTK target is off: valac and GTK 4 were absent')
endif

capi_dir = get_option('capi_dir')
if capi_dir == ''
  error('pass -Dcapi_dir=<cargo target dir>/debug, where libmailune_capi was built')
endif

capi = declare_dependency(
  include_directories: include_directories('../../crates/mailune-capi/include'),
  dependencies: meson.get_compiler('c').find_library('mailune_capi', dirs: capi_dir),
)

linux_inc = include_directories('src')

mailune_linux = static_library(
  'mailune_linux',
  'src/library.c',
  include_directories: linux_inc,
  dependencies: capi,
)

link_test = executable(
  'link-test',
  'tests/link.c',
  include_directories: linux_inc,
  link_with: mailune_linux,
  dependencies: capi,
)

test(
  'link',
  link_test,
  env: {
    'DYLD_LIBRARY_PATH': capi_dir,
    'LD_LIBRARY_PATH': capi_dir,
  },
)

tokens_css = meson.current_source_dir() / 'share' / 'tokens.css'
tokens_test = executable(
  'tokens-test',
  'tests/tokens.c',
  c_args: ['-DTOKENS_CSS="' + tokens_css + '"'],
)
test('tokens', tokens_test)

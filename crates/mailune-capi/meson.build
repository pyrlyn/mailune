# Vala links this crate's cargo-built library, include/mailune.h, and vapi/mailune.vapi.
# nextest does not run Meson and does not open a socket.
project('mailune-capi', 'c', meson_version: '>= 1.0')

capi_dir = get_option('capi_dir')

mailune_capi = declare_dependency(
  dependencies: meson.get_compiler('c').find_library('mailune_capi', dirs: capi_dir, required: false),
  include_directories: include_directories('include'),
  variables: {
    'vapi': meson.current_source_dir() / 'vapi' / 'mailune.vapi',
    'header': meson.current_source_dir() / 'include' / 'mailune.h',
  },
)

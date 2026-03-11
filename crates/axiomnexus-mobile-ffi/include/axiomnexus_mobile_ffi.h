#ifndef AXIOMNEXUS_MOBILE_FFI_H
#define AXIOMNEXUS_MOBILE_FFI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AxiomNexusRuntime AxiomNexusRuntime;

typedef int32_t AxiomNexusFfiCode;
enum {
  AXIOMNEXUS_FFI_CODE_OK = 0,
  AXIOMNEXUS_FFI_CODE_INVALID_ARGUMENT = 1,
  AXIOMNEXUS_FFI_CODE_RUNTIME_ERROR = 2
};

typedef struct AxiomNexusOwnedBytes {
  uint8_t *ptr;
  size_t len;
} AxiomNexusOwnedBytes;

typedef struct AxiomNexusFfiResult {
  AxiomNexusFfiCode code;
  AxiomNexusOwnedBytes payload;
} AxiomNexusFfiResult;

/**
 * Safety contract:
 * - root_dir: non-null UTF-8 C string
 * - out_runtime: non-null writable pointer
 * - runtime must be freed with axiomnexus_runtime_free
 */
AxiomNexusFfiResult axiomnexus_runtime_new(const char *root_dir, AxiomNexusRuntime **out_runtime);

/**
 * Safety contract:
 * - runtime must be a live pointer returned by axiomnexus_runtime_new
 */
AxiomNexusFfiResult axiomnexus_runtime_initialize(AxiomNexusRuntime *runtime);

/**
 * Safety contract:
 * - runtime must be a live pointer returned by axiomnexus_runtime_new
 * - returned payload must be freed with axiomnexus_owned_bytes_free
 */
AxiomNexusFfiResult axiomnexus_runtime_backend_status_json(AxiomNexusRuntime *runtime);

/**
 * Safety contract:
 * - runtime must be a live pointer returned by axiomnexus_runtime_new
 * - uri: non-null UTF-8 C string
 */
AxiomNexusFfiResult axiomnexus_runtime_mkdir(AxiomNexusRuntime *runtime, const char *uri);

/**
 * Safety contract:
 * - runtime must be a live pointer returned by axiomnexus_runtime_new
 * - uri: non-null UTF-8 C string
 */
AxiomNexusFfiResult axiomnexus_runtime_ls_json(
  AxiomNexusRuntime *runtime,
  const char *uri,
  bool recursive
);

/**
 * Safety contract:
 * - runtime must be a live pointer returned by axiomnexus_runtime_new
 * - uri: non-null UTF-8 C string
 */
AxiomNexusFfiResult axiomnexus_runtime_load_markdown_json(
  AxiomNexusRuntime *runtime,
  const char *uri
);

/**
 * Safety contract:
 * - runtime must be a live pointer returned by axiomnexus_runtime_new
 * - uri/content: non-null UTF-8 C strings
 * - expected_etag may be null
 */
AxiomNexusFfiResult axiomnexus_runtime_save_markdown_json(
  AxiomNexusRuntime *runtime,
  const char *uri,
  const char *content,
  const char *expected_etag
);

/**
 * Safety contract:
 * - runtime must be a live pointer returned by axiomnexus_runtime_new
 * - uri: non-null UTF-8 C string
 */
AxiomNexusFfiResult axiomnexus_runtime_rm(
  AxiomNexusRuntime *runtime,
  const char *uri,
  bool recursive
);

/**
 * Safety contract:
 * - runtime must be null or a pointer from axiomnexus_runtime_new
 * - free exactly once
 */
void axiomnexus_runtime_free(AxiomNexusRuntime *runtime);

/**
 * Safety contract:
 * - bytes must come from axiomnexus runtime FFI calls
 * - free exactly once
 */
void axiomnexus_owned_bytes_free(AxiomNexusOwnedBytes bytes);

#ifdef __cplusplus
}
#endif

#endif

#include "miresidian.h"

int main(void) {
    uint32_t (MIRESIDIAN_CALL *abi_version)(void) = miresidian_abi_version;
    const char *(MIRESIDIAN_CALL *invoke)(const char *, const char *, const char *) = miresidian_invoke;

    return (abi_version == 0 || invoke == 0 || MIRESIDIAN_STATUS_OK != 0 || MIRESIDIAN_ABI_VERSION != 5);
}

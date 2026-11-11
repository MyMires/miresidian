#include "../abi/api/miresidian.h"

int main(void) {
    const char *(MIRESIDIAN_CALL *invoke)(const char *) = miresidian_invoke;
    uint32_t (MIRESIDIAN_CALL *version)(void) = miresidian_abi_version;
    return invoke == 0 || version == 0;
}

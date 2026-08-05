/*
 * Copyright 2026 The percore Authors.
 *
 * This project is dual-licensed under Apache 2.0 and MIT terms.
 * See LICENSE-APACHE and LICENSE-MIT for details.
 */

MEMORY
{
	image : ORIGIN = 0x40080000, LENGTH = 2M
}

CORE_COUNT = 2;

SECTIONS
{
	.percore : ALIGN(16) {
		__PERCORE_START__ = .;
		*(SORT_BY_ALIGNMENT(.percore .percore.*))
		/*
		 * Round the section size up to the actual alignment of the section. This ensures that we can
		 * have an array of aligned copies of the .percore section inside the .percore_secondary
		 * section.
		 */
		. = ALIGN(ALIGNOF(.percore));
		__PERCORE_END__ = .;
	} >image

	.percore_secondary (NOLOAD) : ALIGN(ALIGNOF(.percore)) {
		__PERCORE_SECONDARY_START__ = .;
		. += (__PERCORE_END__ - __PERCORE_START__) * (CORE_COUNT - 1);
		__PERCORE_SECONDARY_END__ = .;
	} >image
}

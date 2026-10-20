<template>
  <div class="flex min-h-screen flex-col bg-surface-canvas font-outfit text-ink-primary">
    <SiteHeader />
    <div class="flex flex-1 flex-col">
      <slot />
    </div>
    <SiteFooter />
  </div>
</template>

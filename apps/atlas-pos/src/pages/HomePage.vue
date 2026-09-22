<script setup lang="ts">
import {
  IonContent,
  IonHeader,
  IonPage,
  IonTitle,
  IonToolbar,
} from '@ionic/vue';
</script>

<template>
  <IonPage>
    <IonHeader>
      <IonToolbar>
        <IonTitle>Atlas POS</IonTitle>
      </IonToolbar>
    </IonHeader>
    <IonContent class="ion-padding">
      <!-- Placeholder route - scaffold-only pass, no business logic or
           @atlas/ui-app components wired yet. First real screen is checkout
           once the offline-sync API contract exists. -->
      <p>atlas-pos scaffold.</p>
    </IonContent>
  </IonPage>
</template>

import '../app.css';
import { mount } from 'svelte';
import Gallery from './Gallery.svelte';

mount(Gallery, {
  target: document.getElementById('gallery')!,
});

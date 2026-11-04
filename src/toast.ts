import { reactive } from 'vue'

export const toastState = reactive({
  message: '',
  visible: false,
})

let timer = 0

export function showToast(message: string) {
  toastState.message = message
  toastState.visible = true
  window.clearTimeout(timer)
  timer = window.setTimeout(() => {
    toastState.visible = false
  }, 2400)
}

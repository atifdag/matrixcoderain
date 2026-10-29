/* Matrix - Kod Yağmuru: C üzerinden çağrı sözleşmesi, sürüm 1.
 *
 * Sonuç kodları:
 *   0 başarılı, 1 geçersiz girdi, 2 geçersiz motor kimliği,
 *   3 kısa görüntü alanı, 4 çizim durdurulmuş, 5 yanlış iş parçacığı,
 *   6 beklenmeyen hata.
 *
 * Bir motorun bütün çağrıları onu oluşturan iş parçacığından yapılmalıdır.
 * Verilen adreslerin geçerliliği ve çağrı boyunca yaşaması çağıranın
 * sorumluluğudur. */
#ifndef MATRIX_CODERAIN_H
#define MATRIX_CODERAIN_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

/* Kitaplığın desteklediği sözleşme sürümü. matrix_create çağrısına verilir. */
uint32_t matrix_abi_version(void);

/* Motoru durdurulmuş olarak oluşturur ve kimliğini result adresine yazar.
   font_len sıfırsa programa gömülü yazı tipleri kullanılır. */
int32_t matrix_create(uint32_t version, uint32_t width, uint32_t height,
    uint64_t seed, const uint8_t *font, size_t font_len, uint64_t *result);

/* active: 1 çizimi başlatır, 0 durdurur. */
int32_t matrix_set_active(uint64_t handle, uint32_t active);

/* Geçen süreyi mikrosaniye olarak ve ekrandaki saati verir. */
int32_t matrix_update(uint64_t handle, uint64_t elapsed_us,
    uint8_t hour, uint8_t minute, uint8_t second);

/* Görüntü boyutunu değiştirir. Geçersiz boyutta eski boyut korunur. */
int32_t matrix_resize(uint64_t handle, uint32_t width, uint32_t height);

/* Görüntüyü yazar. Her piksel bellekte kırmızı, yeşil, mavi, 255 sırasıyla
   dört bayttır. stride bir satırın bayt uzunluğudur ve en az width*4 olmalıdır;
   satır sonundaki boşluklara dokunulmaz. len son satırın son pikselini
   kapsamalıdır. */
int32_t matrix_render_rgba(uint64_t handle, uint8_t *target, size_t len, size_t stride);

/* Sayaçları ve son karelerin çizim sürelerini Türkçe tek satır olarak yazar.
   Metin UTF-8'dir, sonuna sıfır eklenir; 256 baytlık alan her zaman yeterlidir. */
int32_t matrix_stats_text(uint64_t handle, uint8_t *text, size_t len);

/* Motoru kapatır. Aynı kimlik tekrar kapatılırsa 2 döner. */
int32_t matrix_destroy(uint64_t handle);

#ifdef __cplusplus
}
#endif
#endif

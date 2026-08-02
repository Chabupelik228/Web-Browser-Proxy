// frontend/src/composables/useScramjetStorage.js
const SCRAMJET_DB_NAME = '$scramjet';
const SCRAMJET_COOKIE_STORE = 'cookies';

function openScramjetDb() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(SCRAMJET_DB_NAME);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
    // не создаём структуру сами — её создаёт сам Scramjet при инициализации.
    // Если её вдруг ещё нет, просто откроется пустая/старая версия базы.
  });
}

// Забираем ВСЕ куки в виде { key: value } — для шифрования и отправки на сервер
export async function exportScramjetCookies() {
  try {
    const db = await openScramjetDb();
    if (!db.objectStoreNames.contains(SCRAMJET_COOKIE_STORE)) {
      db.close();
      return {};
    }
    return await new Promise((resolve, reject) => {
      const tx = db.transaction(SCRAMJET_COOKIE_STORE, 'readonly');
      const store = tx.objectStore(SCRAMJET_COOKIE_STORE);
      const result = {};
      const cursorReq = store.openCursor();
      cursorReq.onsuccess = (e) => {
        const cursor = e.target.result;
        if (cursor) {
          result[cursor.key] = cursor.value;
          cursor.continue();
        } else {
          db.close();
          resolve(result);
        }
      };
      cursorReq.onerror = () => {
        db.close();
        reject(cursorReq.error);
      };
    });
  } catch (e) {
    console.error('[ScramjetStorage] Не удалось прочитать куки:', e);
    return {};
  }
}

export async function importScramjetCookies(cookiesObj, attempt = 0) {
  if (!cookiesObj || typeof cookiesObj !== 'object' || Object.keys(cookiesObj).length === 0) return;

  try {
    const db = await openScramjetDb();
    if (!db.objectStoreNames.contains(SCRAMJET_COOKIE_STORE)) {
      db.close();
      if (attempt < 3) {
        await new Promise(r => setTimeout(r, 400));
        return importScramjetCookies(cookiesObj, attempt + 1);
      }
      console.warn('[ScramjetStorage] Store cookies так и не появился, восстановление пропущено');
      return;
    }

    await new Promise((resolve, reject) => {
      const tx = db.transaction(SCRAMJET_COOKIE_STORE, 'readwrite');
      const store = tx.objectStore(SCRAMJET_COOKIE_STORE);
      for (const [key, value] of Object.entries(cookiesObj)) {
        store.put(value, key);
      }
      tx.oncomplete = () => { db.close(); resolve(); };
      tx.onerror = () => { db.close(); reject(tx.error); };
    });
  } catch (e) {
    console.error('[ScramjetStorage] Не удалось записать куки:', e);
  }
}
Hola. Vamos a trabajar con Pillbox MCP para gestionar el conocimiento de un proyecto nuevo.

Antes de empezar, una cosa: cuando trabajemos juntos, prefiero que los títulos de las pills sean siempre en español y que sean cortos y descriptivos — máximo 8 palabras. Es algo que aplico en todos mis proyectos, así que tenlo en cuenta para el futuro.

El proyecto se llama **"Recetas del chef"**. Es un sistema de gestión de recetas de cocina con un backend en Node.js y una base de datos SQLite. Vamos a registrarlo en Pillbox y a empezar a capturar conocimiento sobre él.

Primero, registra el bottle para este proyecto. Usa:
- nombre: `recetas-del-chef`
- display_name: `Recetas del chef`
- directorio: `/tmp/recetas-del-chef`
- scope: local

Una vez registrado, abre una nueva prescription con título: **"Diseño inicial del sistema de recetas"**

---

Perfecto. Ahora guarda las siguientes pills en esa prescription:

**Pill 1** — compound `decision`
Hemos decidido usar SQLite con WAL mode para la base de datos. La razón es que el sistema es single-user y no necesitamos concurrencia real. Se descartó PostgreSQL por ser excesivo para el alcance del proyecto.

**Pill 2** — compound `discovery`
El esquema inicial tiene tres tablas: `recipes` (id, name, description, created_at), `ingredients` (id, recipe_id, name, quantity, unit) y `steps` (id, recipe_id, order, instruction).

**Pill 3** — compound `specification`
Regla de validación: un ingrediente no puede tener `quantity` negativa. Si la API recibe un valor negativo debe rechazarlo con HTTP 422 y el mensaje `"quantity must be positive"`.

**Pill 4** — compound `discovery`
Durante la exploración encontramos que el cliente quiere que las recetas se puedan marcar como "favoritas". Esto no estaba en el brief original — hay que revisar el modelo de datos.

---

Ahora necesito hacer algunas correcciones:

1. La pill sobre el esquema (Pill 2) está incompleta. Revísala y añade al contenido: "Tabla adicional `favorites` (user_id, recipe_id, created_at) pendiente de confirmar con el cliente."

2. La pill sobre favoritos (Pill 4) fue un malentendido — no es del proyecto actual sino de otro. Descártala (soft-delete).

3. Haz una búsqueda de pills con el término "favorit" para confirmar que la pill descartada ya no aparece en los resultados normales.

---

Cierra la prescription.

Ahora abre una segunda prescription con título: **"Validaciones y errores API"**

Guarda en ella dos pills:

**Pill A** — compound `specification`
Todos los endpoints de la API deben devolver errores en formato JSON `{"error": "mensaje"}`. No se permiten respuestas de error en texto plano.

**Pill B** — compound `task`
Implementar middleware de validación de errores en Express. Debe capturar errores de Zod y formatearlos al estándar definido.

---

Descarta esta segunda prescription completa (soft-delete en cascada).

Después de descartarla, usa `bottle_context` para listar todas las prescriptions del bottle "Recetas del chef" y dime cuántas hay y en qué estado está cada una.

---

Ahora cambiemos de tema. Tengo una preferencia general que quiero que tengas en cuenta: cuando trabajo en proyectos de backend en Node.js, siempre uso Zod para validación de esquemas — es mi librería estándar para eso. No hace falta que me lo preguntes en el futuro.

Crea una capsule sobre las preferencias de estilo de código que tengo: uso siempre `async/await` en lugar de callbacks o `.then()` en JavaScript/TypeScript. Es una convención que aplico en todos mis proyectos.

---

Ahora realiza una búsqueda FTS de pills con el término "validación" y otra con "SQLite". Dime qué encuentras en cada caso — incluye título, compound y prescription de cada resultado.

---

Haz un `pill_search` de cualquier pill que mencione "favorit" (sin acento). Verifica que:
- La pill descartada de la primera prescription NO aparece.
- Si ves algo, explica qué es.

---

Por último, crea una capsule sobre este proyecto (si no lo has hecho ya): has aprendido que el usuario trabaja en un sistema de recetas de cocina con Node.js + SQLite y que prefiere proyectos con alcance acotado y bien definido antes de empezar a implementar.

Espera — antes de crearla, busca primero si ya existe alguna capsule similar para no duplicar.

---

Resume todo lo que has hecho en esta sesión: bottles creados, prescriptions (estado de cada una), pills guardadas/descartadas/revisadas, y capsules creadas o buscadas.
use anyhow::Result;
use serde_json::json;
use sqlx::PgPool;

pub async fn seed_defaults(pool: &PgPool) -> Result<()> {
    seed_categories(pool).await?;
    seed_articles(pool).await?;
    seed_guides(pool).await?;
    Ok(())
}

async fn seed_categories(pool: &PgPool) -> Result<()> {
    let rows = [
        (
            "Вид на жительство",
            "residencia",
            "Типы ВНЖ и общая логика подачи.",
            "badge",
            10,
        ),
        (
            "Семья гражданина Испании / ЕС",
            "familia-ue",
            "Базовые материалы о семейных процедурах.",
            "users",
            20,
        ),
        (
            "Arraigo",
            "arraigo",
            "Обзор процедур arraigo без юридических гарантий.",
            "landmark",
            30,
        ),
        (
            "Nacionalidad española",
            "nacionalidad",
            "Гражданство Испании: общие шаги и источники.",
            "flag",
            40,
        ),
        (
            "Cita previa",
            "cita-previa",
            "Запись на прием и подготовка к визиту.",
            "calendar",
            50,
        ),
        (
            "Empadronamiento",
            "empadronamiento",
            "Регистрация по месту проживания.",
            "home",
            60,
        ),
        (
            "Медицинская система",
            "salud",
            "Система salud и практические вопросы.",
            "heart",
            70,
        ),
        (
            "Школы и образование",
            "educacion",
            "Школы, документы и адаптация детей.",
            "school",
            80,
        ),
        (
            "Банки",
            "bancos",
            "Счета, карты и базовые банковские вопросы.",
            "wallet",
            90,
        ),
        (
            "Налоги",
            "impuestos",
            "Налоговые темы для новичков.",
            "receipt",
            100,
        ),
        (
            "Работа",
            "trabajo",
            "Работа, контракты и права работника.",
            "briefcase",
            110,
        ),
        (
            "Автомобиль и права",
            "coche-y-carnet",
            "Права, авто и бытовые процедуры.",
            "car",
            120,
        ),
        (
            "Аренда жилья",
            "alquiler",
            "Аренда, договоры и документы.",
            "key",
            130,
        ),
        (
            "Документы и переводы",
            "documentos-traducciones",
            "Переводы, копии и подготовка пакетов.",
            "file-text",
            140,
        ),
        (
            "Апостиль",
            "apostilla",
            "Когда может понадобиться апостиль.",
            "stamp",
            150,
        ),
        (
            "Практическая жизнь в Испании",
            "vida-practica",
            "Повседневные вопросы после переезда.",
            "map",
            160,
        ),
    ];

    for (title, slug, description, icon, sort_order) in rows {
        sqlx::query(
            r#"
            INSERT INTO categories (title_ru, slug, description_ru, icon, sort_order)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (slug) DO NOTHING
            "#,
        )
        .bind(title)
        .bind(slug)
        .bind(description)
        .bind(icon)
        .bind(sort_order)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn seed_guides(pool: &PgPool) -> Result<()> {
    let guides = [
        (
            "Как подготовиться к cita previa",
            "kak-podgotovitsya-k-cita-previa",
            "Безопасный чеклист подготовки к записи и визиту.",
        ),
        (
            "Что такое empadronamiento",
            "chto-takoe-empadronamiento",
            "Общее объяснение регистрации по адресу в Испании.",
        ),
        (
            "Какие документы часто требуют в Extranjería",
            "dokumenty-extranjeria",
            "Типовые группы документов без финальных юридических требований.",
        ),
        (
            "Как пользоваться ИИ-помощником",
            "kak-polzovatsya-ai-pomoshchnikom",
            "Как задавать вопросы и проверять источники.",
        ),
        (
            "Как собирать документы для подачи",
            "kak-sobirat-dokumenty-dlya-podachi",
            "Практичный порядок подготовки пакета документов.",
        ),
    ];

    for (title, slug, summary) in guides {
        sqlx::query(
            r#"
            INSERT INTO guides (
              title_ru, slug, summary_ru, target_audience, required_documents, steps,
              deadlines, fees, where_to_submit, common_mistakes, risks, official_sources
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (slug) DO NOTHING
            "#,
        )
        .bind(title)
        .bind(slug)
        .bind(summary)
        .bind("Новички в Испании. Требования зависят от процедуры и региона.")
        .bind(json!(["Паспорт или NIE, если применимо", "Документы по конкретной процедуре", "Копии и переводы, если это требует официальный орган"]))
        .bind(json!(["Проверьте официальный источник", "Соберите документы", "Сделайте копии", "Сохраните подтверждение записи или подачи"]))
        .bind(json!(["Сроки зависят от процедуры. Проверяйте дату начала срока в официальном источнике."]))
        .bind(json!(["Пошлины зависят от процедуры и формы. Не используйте этот MVP как источник финальной суммы."]))
        .bind("Официальный орган, указанный для конкретной процедуры.")
        .bind(json!(["Опираться на устаревшие списки", "Не проверять региональные требования", "Приносить только оригиналы без копий"]))
        .bind(json!(["Процедуры меняются", "Для рискованных случаев нужен официальный источник или лицензированный специалист"]))
        .bind(json!([]))
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn seed_articles(pool: &PgPool) -> Result<()> {
    let articles = [
        (
            "cita-previa",
            "Что проверить перед cita previa",
            "chto-proverit-pered-cita-previa",
            "Безопасный список подготовки к записи без финальных юридических требований.",
            "Перед визитом проверьте официальный сайт органа, адрес, дату, время, тип процедуры и список документов. Возьмите оригиналы и копии, если это указано в официальной инструкции. Этот материал не заменяет требования конкретной процедуры.",
        ),
        (
            "empadronamiento",
            "Empadronamiento: зачем он может понадобиться",
            "empadronamiento-zachem-nuzhen",
            "Общее объяснение роли регистрации по адресу в Испании.",
            "Empadronamiento часто используют как подтверждение проживания по адресу. Конкретные требования зависят от муниципалитета и процедуры. Проверяйте сайт ayuntamiento перед подачей.",
        ),
        (
            "documentos-traducciones",
            "Как хранить пакет документов",
            "kak-hranit-paket-dokumentov",
            "Практичный способ не потерять важные копии и подтверждения.",
            "Сохраняйте сканы, копии, квитанции, подтверждения записи и подачи. Для юридически значимых действий проверяйте, нужна ли заверенная копия, перевод или апостиль в официальном источнике.",
        ),
    ];

    for (category_slug, title, slug, summary, body) in articles {
        sqlx::query(
            r#"
            INSERT INTO articles (
              category_id, title_ru, slug, summary_ru, body_ru_markdown,
              tags, source_references, legal_risk_level, is_published, include_in_ai
            )
            SELECT id, $1, $2, $3, $4, $5, '[]', 'low', true, true
            FROM categories
            WHERE slug = $6
            ON CONFLICT (slug) DO NOTHING
            "#,
        )
        .bind(title)
        .bind(slug)
        .bind(summary)
        .bind(body)
        .bind(vec!["demo".to_owned(), "безопасная-информация".to_owned()])
        .bind(category_slug)
        .execute(pool)
        .await?;
    }

    Ok(())
}

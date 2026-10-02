use super::*;
use crate::Db;
use serde_json::json;

#[tokio::test]
async fn t5_utf8_relative_order_and_missing_m3u_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let album = dir.path().join("Álbum");
    std::fs::create_dir_all(&album).unwrap();
    let first = album.join("Música.flac");
    let second = album.join("Segunda.opus");
    std::fs::write(&first, b"audio").unwrap();
    std::fs::write(&second, b"audio").unwrap();
    let root = dir.path().to_owned();
    let db = Db::open_in_memory().unwrap();
    let content=db.call(move|conn| {
        let sync:Sync=serde_json::from_value(json!({"id":"m3u","provider":"youtube","url":"https://www.youtube.com/playlist?list=test","playlistId":"test","title":"Minha playlist","thumbnail":null,"profileId":"original","outputDir":root,"intervalHours":0,"maxItems":null,"removeDeleted":false,"writeM3u":true,"enabled":true,"lastSyncAt":null,"lastResult":null,"createdAt":1,"itemCount":4}))?;
        conn.execute("INSERT INTO syncs(id,url,title,profile_id,created_at) VALUES('m3u','url','Minha playlist','original',1)",[])?;
        for (id,path,title,artist,duration,missing) in [(1,first,"Música","Björk",10.4,false),(2,second,"Segunda","Artista",20.6,false),(3,root.join("absent.opus"),"Ausente","Artista",30.0,true),(4,root.join("removed.opus"),"Removida","Artista",40.0,false)] {
            conn.execute("INSERT INTO library(id,file_path,title,artist,duration_s,missing,added_at,updated_at) VALUES(?,?,?,?,?,?,1,1)",params![id,path.to_string_lossy(),title,artist,duration,missing])?;
            conn.execute("INSERT INTO sync_items(sync_id,source_id,position,state,library_id,first_seen_at) VALUES('m3u',?,?,?, ?,1)",params![id.to_string(),if id==1 {2} else if id==2 {1} else {id},if id==4 {"removed"} else {"present"},id])?;
        }
        let output=write_m3u(conn,&sync,&Settings::default())?;
        let text=std::fs::read_to_string(&output)?;
        // Regeneration replaces the previous playlist instead of appending.
        std::fs::write(&output,"old")?;write_m3u(conn,&sync,&Settings::default())?;
        assert_eq!(std::fs::read_to_string(&output)?,text);
        Ok(text)
    }).await.unwrap();
    assert!(content.ends_with('\n'));
    insta::assert_snapshot!(content, @r"
    #EXTM3U
    #EXTINF:21,Artista - Segunda
    ../Álbum/Segunda.opus
    #EXTINF:10,Björk - Música
    ../Álbum/Música.flac
    ");
}

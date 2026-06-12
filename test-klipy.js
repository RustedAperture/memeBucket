const http = require('http');

async function test() {
  const q1 = await fetch('http://localhost:3000/api/gifs/search?q=cat&page=1').then(r => r.json()).catch(() => null);
  const q2 = await fetch('http://localhost:3000/api/gifs/search?q=cat&page=2').then(r => r.json()).catch(() => null);
  
  if (!q1 || !q2) {
    console.log("Failed to fetch. Server might not be running.");
    return;
  }
  
  const id1 = q1.data?.data?.[0]?.id || q1.data?.[0]?.id;
  const id2 = q2.data?.data?.[0]?.id || q2.data?.[0]?.id;
  
  console.log("Page 1 first ID:", id1);
  console.log("Page 2 first ID:", id2);
  console.log("Are they the same?", id1 === id2);
}
test();

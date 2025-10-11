#!/usr/bin/env node
/**
 * Test script to validate frontend API integration with new backend
 */

const API_BASE = 'http://localhost:8080/v1';

async function testAPI() {
  console.log('🧪 Testing API Integration with NQRust-MicroVM Backend\n');

  const tests = [
    { name: 'List VMs', endpoint: '/vms' },
    { name: 'List Templates', endpoint: '/templates' },
    { name: 'List Images', endpoint: '/images' },
  ];

  for (const test of tests) {
    try {
      console.log(`⏳ Testing: ${test.name}`);
      const response = await fetch(API_BASE + test.endpoint);

      if (!response.ok) {
        console.log(`❌ ${test.name}: HTTP ${response.status}`);
        continue;
      }

      const data = await response.json();
      console.log(`✅ ${test.name}: ${data.items ? data.items.length : 0} items`);

      if (data.items && data.items.length > 0) {
        console.log(`   Sample item keys: ${Object.keys(data.items[0]).join(', ')}`);
      }
    } catch (error) {
      console.log(`❌ ${test.name}: ${error.message}`);
    }
  }

  // Test creating a template
  console.log('\n⏳ Testing: Create Template');
  try {
    const templateData = {
      name: 'test-template-' + Date.now(),
      spec: {
        vcpu: 1,
        mem_mib: 256,
        kernel_path: '/path/to/kernel',
        rootfs_path: '/path/to/rootfs'
      }
    };

    const response = await fetch(API_BASE + '/templates', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(templateData)
    });

    if (response.ok) {
      const result = await response.json();
      console.log(`✅ Create Template: Created with ID ${result.id}`);

      // Clean up - delete the template
      await fetch(API_BASE + `/templates/${result.id}`, { method: 'DELETE' }).catch(() => {});
    } else {
      console.log(`❌ Create Template: HTTP ${response.status}`);
    }
  } catch (error) {
    console.log(`❌ Create Template: ${error.message}`);
  }

  console.log('\n🎉 API Integration test complete!');
}

testAPI().catch(console.error);
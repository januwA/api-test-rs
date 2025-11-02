import { Table, Button, Input, Checkbox, Space, Empty } from "antd";
import { PlusOutlined, DeleteOutlined, ClearOutlined } from "@ant-design/icons";
import { PairUi } from "../../types";

interface PairTableProps {
  data: PairUi[];
  onChange: (data: PairUi[]) => void;
  placeholder?: { key: string; value: string };
}

function PairTable({ data, onChange, placeholder }: PairTableProps) {
  const handleAdd = () => {
    onChange([...data, { key: "", value: "", disable: false }]);
  };

  const handleDelete = (index: number) => {
    onChange(data.filter((_, i) => i !== index));
  };

  const handleUpdate = (index: number, field: keyof PairUi, value: any) => {
    const newData = [...data];
    newData[index] = { ...newData[index], [field]: value };
    onChange(newData);
  };

  const columns = [
    {
      title: "启用",
      dataIndex: "disable",
      key: "enable",
      width: 60,
      align: "center" as const,
      render: (_: any, row: any, index: number) => (
        <Checkbox
          checked={!row.disable}
          onChange={(e) => handleUpdate(index, "disable", !e.target.checked)}
        />
      ),
    },
    {
      title: placeholder?.key || "Key",
      dataIndex: "key",
      key: "key",
      render: (_: any, row: any, index: number) => (
        <Input
          value={row.key}
          onChange={(e) => handleUpdate(index, "key", e.target.value)}
          placeholder={placeholder?.key || "键"}
          size="small"
        />
      ),
    },
    {
      title: placeholder?.value || "Value",
      dataIndex: "value",
      key: "value",
      render: (_: any, row: any, index: number) => (
        <Input
          value={row.value}
          onChange={(e) => handleUpdate(index, "value", e.target.value)}
          placeholder={placeholder?.value || "值"}
          size="small"
        />
      ),
    },
    {
      title: "操作",
      key: "action",
      width: 80,
      align: "center" as const,
      render: (_: any, __: any, index: number) => (
        <Button
          type="link"
          danger
          size="small"
          icon={<DeleteOutlined />}
          onClick={() => handleDelete(index)}
        >
          删除
        </Button>
      ),
    },
  ];

  return (
    <div>
      <Space style={{ marginBottom: 16 }}>
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={handleAdd}
          size="small"
        >
          添加
        </Button>
        {data.length > 0 && (
          <Button
            danger
            icon={<ClearOutlined />}
            onClick={() => onChange([])}
            size="small"
          >
            清空
          </Button>
        )}
      </Space>

      <Table
        columns={columns}
        dataSource={data.map((item, index) => ({ ...item, tableKey: index.toString() }))}
        pagination={false}
        size="small"
        rowKey="tableKey"
        locale={{
          emptyText: <Empty description='暂无数据，点击"添加"按钮添加' image={Empty.PRESENTED_IMAGE_SIMPLE} />,
        }}
      />
    </div>
  );
}

export default PairTable;

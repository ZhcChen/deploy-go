//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'external_api_key_summary.g.dart';

/// ExternalApiKeySummary
///
/// Properties:
/// * [applicationIds]
/// * [createdAt]
/// * [expiresAt]
/// * [id]
/// * [lastUsedAt]
/// * [name]
/// * [status]
/// * [updatedAt]
/// * [version]
@BuiltValue()
abstract class ExternalApiKeySummary implements Built<ExternalApiKeySummary, ExternalApiKeySummaryBuilder> {
  @BuiltValueField(wireName: r'application_ids')
  BuiltList<String> get applicationIds;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'expires_at')
  String? get expiresAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'last_used_at')
  String? get lastUsedAt;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'version')
  int get version;

  ExternalApiKeySummary._();

  factory ExternalApiKeySummary([void updates(ExternalApiKeySummaryBuilder b)]) = _$ExternalApiKeySummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ExternalApiKeySummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ExternalApiKeySummary> get serializer => _$ExternalApiKeySummarySerializer();
}

class _$ExternalApiKeySummarySerializer implements PrimitiveSerializer<ExternalApiKeySummary> {
  @override
  final Iterable<Type> types = const [ExternalApiKeySummary, _$ExternalApiKeySummary];

  @override
  final String wireName = r'ExternalApiKeySummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ExternalApiKeySummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_ids';
    yield serializers.serialize(
      object.applicationIds,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.expiresAt != null) {
      yield r'expires_at';
      yield serializers.serialize(
        object.expiresAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.lastUsedAt != null) {
      yield r'last_used_at';
      yield serializers.serialize(
        object.lastUsedAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ExternalApiKeySummary object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ExternalApiKeySummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.applicationIds.replace(valueDes);
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.expiresAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'last_used_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.lastUsedAt = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ExternalApiKeySummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ExternalApiKeySummaryBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
